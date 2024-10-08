use base64::{engine::general_purpose::STANDARD as base64_encoder, Engine};
use image::{DynamicImage, GenericImage, ImageFormat, ImageReader, ImageResult, RgbaImage};

use std::{
    collections::HashMap,
    error::Error,
    fmt,
    io::{Cursor, Seek, Write},
    ops::Deref,
    path::Path,
};

use crate::utils;

#[derive(Debug, Clone)]
pub struct DynamicImageWithFormat {
    data: DynamicImage,
    format: image::ImageFormat,
}

impl Deref for DynamicImageWithFormat {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl TryFrom<rust_embed::EmbeddedFile> for DynamicImageWithFormat {
    type Error = Box<dyn Error>;
    fn try_from(value: rust_embed::EmbeddedFile) -> Result<Self, Self::Error> {
        let bytes = Cursor::new(value.data.as_ref());
        let reader = ImageReader::new(bytes).with_guessed_format()?;
        let format = reader.format().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unable to detect image format",
        ))?;
        let data = reader.decode()?;

        Ok(DynamicImageWithFormat { data, format })
    }
}

impl DynamicImageWithFormat {
    pub fn open<P>(path: P) -> ImageResult<Self>
    where
        P: AsRef<std::path::Path>,
    {
        // get format
        let reader = ImageReader::open(path)?.with_guessed_format()?;
        let format = reader.format().unwrap();
        let data = reader.decode()?;

        Ok(DynamicImageWithFormat { data, format })
    }

    pub fn as_raw(&self) -> &DynamicImage {
        &self.data
    }

    pub fn write_to<W>(&self, w: &mut W) -> ImageResult<()>
    where
        W: Write + Seek,
    {
        self.data.write_to(w, self.format)
    }

    pub fn encode(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut buffer = Cursor::new(Vec::new());

        self.data.write_to(&mut buffer, self.format)?;

        Ok(buffer.into_inner())
    }

    pub fn format(&self) -> ImageFormat {
        self.format.clone()
    }
}

#[derive(Debug, Clone)]
pub struct SvgImage {
    width: u32,
    height: u32,
    data: String,
}

impl SvgImage {
    pub fn data(&self) -> &str {
        &self.data
    }
}

impl From<&DynamicImageWithFormat> for SvgImage {
    fn from(value: &DynamicImageWithFormat) -> Self {
        let mut buffer = Cursor::new(Vec::new());
        value.write_to(&mut buffer).unwrap();

        let data = buffer.into_inner();
        let encoded_data = base64_encoder.encode(data);

        let data = format!(
            "data:{};charset=utf-8;base64,{}",
            value.format.to_mime_type(),
            encoded_data
        );

        SvgImage {
            width: value.width(),
            height: value.height(),
            data,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    digits: HashMap<u32, DynamicImageWithFormat>,
    svg_digits: HashMap<u32, SvgImage>,
}

impl Theme {
    fn new(digits: HashMap<u32, DynamicImageWithFormat>) -> Self {
        let mut svg_digits = HashMap::new();
        for (key, val) in digits.iter() {
            svg_digits.insert(*key, val.into());
        }
        Theme { digits, svg_digits }
    }

    pub fn gen_webp(&self, number: u64, digits_count: u32) -> ImageResult<DynamicImageWithFormat> {
        let number_digits = utils::u64_to_digit(number, digits_count);

        let mut multparts = Vec::new();
        let mut height = 0;
        let mut width = 0;

        for digit in number_digits {
            // digit must be exist
            let digit = self.digits.get(&digit).unwrap();
            let digit_width = digit.width();
            let digit_height = digit.height();

            multparts.push((width, digit));
            height = height.max(digit_height);
            width += digit_width;
        }

        let mut concated_img = RgbaImage::new(width, height);

        for (x, digit) in multparts {
            concated_img.copy_from(digit.as_raw(), x, 0)?;
        }

        Ok(DynamicImageWithFormat {
            format: image::ImageFormat::WebP,
            data: DynamicImage::ImageRgba8(concated_img),
        })
    }

    pub fn gen_svg(
        &self,
        number: u64,
        digits_count: u32,
        pixelated: bool,
    ) -> ImageResult<SvgImage> {
        // convert u32 to digits vector with extra digit
        let number_digits = utils::u64_to_digit(number, digits_count);

        let mut multparts = String::new();
        let mut height = 0;
        let mut width = 0;

        for digit in number_digits {
            // digit must be exist
            let digit = self.svg_digits.get(&digit).unwrap();

            let digit_width = digit.width;
            let digit_height = digit.height;
            let data = &digit.data;

            multparts.push_str(&format!("<image x=\"{width}\" y=\"0\" width=\"{digit_width}\" height=\"{digit_height}\" href=\"{data}\" />\n"));

            width += digit.width;
            height = height.max(digit.height);
        }

        let mut svg_payload = String::new();
        svg_payload.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        svg_payload.push_str(&format!("<svg width=\"{width}\" height=\"{height}\" version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\""));

        if pixelated {
            svg_payload.push_str(" style='image-rendering: pixelated;'");
        }

        svg_payload.push_str(">\n");
        svg_payload.push_str(&format!("<title>{}</title>\n", number));
        svg_payload.push_str(&format!("<g>{multparts}</g>\n"));
        svg_payload.push_str("</svg>");

        Ok(SvgImage {
            width,
            height,
            data: svg_payload,
        })
    }
}

#[derive(rust_embed::Embed)]
#[folder = "themes/"]
struct ThemeAssets;

#[derive(Debug, Clone)]
pub struct ThemeManager {
    themes_dir: String,
    themes: HashMap<String, Theme>,
}

impl ThemeManager {
    pub fn new(themes_dir: &str) -> std::io::Result<Self> {
        let mut themes = HashMap::new();

        let themes_from_internal = Self::load_themes_from_internal();
        let themes_from_external = Self::load_themes_from_external(themes_dir);

        match themes_from_internal {
            Ok(assets) => themes.extend(assets),
            Err(e) => println!("[Warn] Failed to load internal assets {:?}", e),
        };
        match themes_from_external {
            Ok(assets) => themes.extend(assets),
            Err(e) => println!("[Warn] Failed to load external assets {:?}", e),
        };

        let theme_manager = ThemeManager {
            themes_dir: themes_dir.to_string(),
            themes,
        };
        Ok(theme_manager)
    }

    fn load_themes_from_internal() -> std::io::Result<HashMap<String, Theme>> {
        let mut assets: HashMap<String, HashMap<u32, DynamicImageWithFormat>> = HashMap::new();

        // iter embed assets
        for file_path in ThemeAssets::iter() {
            // assumption: the path is <theme_name>/<digit>.ext

            let path = std::path::Path::new(file_path.as_ref());
            let mut path_iter = path.components().rev();
            let file_name = path.file_stem();

            let _ = path_iter.next();
            let theme_name = path_iter.next();
            if theme_name.is_none() || file_name.is_none() {
                continue;
            }

            let file_name = file_name.unwrap();

            let digit = file_name.to_string_lossy().parse();
            if digit.is_err() {
                continue;
            }

            let digit: u32 = digit.unwrap();

            // load image
            let theme_name = theme_name.unwrap().as_os_str().to_string_lossy();

            // init hashmap
            let themes = assets.get(theme_name.as_ref());
            if themes.is_none() {
                assets.insert(theme_name.to_string(), HashMap::new());
            }
            let themes = assets.get_mut(theme_name.as_ref()).unwrap();
            let image = ThemeAssets::get(file_path.as_ref()).unwrap().try_into();

            if image.is_err() {
                continue;
            }

            let image = image.unwrap();

            themes.insert(digit, image);
        }

        // check all themes
        let mut themes = HashMap::new();
        for (theme_name, digits) in assets.drain() {
            if digits.len() == 10 {
                themes.insert(theme_name, Theme::new(digits));
            }
        }

        Ok(themes)
    }

    fn load_themes_from_external(themes_dir: &str) -> std::io::Result<HashMap<String, Theme>> {
        let mut themes = HashMap::new();

        // iter themes_dir to found all avaliable theme
        // check path
        if !Path::new(themes_dir).try_exists()? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                themes_dir,
            ));
        }

        let sub_dirs = std::fs::read_dir(themes_dir)?;
        for entry in sub_dirs {
            if entry.is_err() {
                continue;
            }
            // must be ok
            let entry = entry.unwrap();

            // skip file
            if !entry.file_type().unwrap().is_dir() {
                continue;
            }

            // so now all entry is a dir represent as a theme
            let theme_name = entry.file_name().into_string().unwrap(); // on must OS, this should be fine

            // collect all image
            let mut theme_images: HashMap<u32, DynamicImageWithFormat> = HashMap::new();
            let mut theme_path = std::path::PathBuf::new();
            theme_path.push(themes_dir);
            theme_path.push(&theme_name);

            let mut digit_img_count = 0;
            for entry in std::fs::read_dir(theme_path.as_path())? {
                if entry.is_err() {
                    break;
                }
                let entry = entry.unwrap();
                let image = DynamicImageWithFormat::open(entry.path());
                if image.is_err() {
                    break;
                }
                let image_path = entry.path();
                let image_name = image_path.file_stem();
                if image_name.is_none() {
                    break;
                }
                let image_name = image_name.unwrap();
                let digit = image_name.to_str().unwrap().parse::<u32>();
                if digit.is_err() {
                    break;
                }

                let image = image.unwrap();
                let digit = digit.unwrap();

                theme_images.insert(digit, image);
                digit_img_count += 1;
            }
            // bad theme, skip
            if digit_img_count != 10 {
                continue;
            }

            // add this theme to manager
            let theme = Theme::new(theme_images);
            themes.insert(theme_name, theme);
        }

        Ok(themes)
    }
    pub fn get(&self, theme_name: &str) -> std::io::Result<&Theme> {
        match self.themes.get(theme_name) {
            Some(theme) => Ok(theme),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                theme_name,
            )),
        }
    }
}

impl fmt::Display for ThemeManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ThemeManager: {}", self.themes_dir)?;

        let mut print_out = String::new();
        for theme_name in self.themes.keys() {
            print_out.push_str(&format!("  {}\n", theme_name));
        }
        write!(f, "{}", print_out)
    }
}
