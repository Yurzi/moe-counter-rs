# Moe-Counter-Rs

使用 Rust 实现的，可跨平台的多种风格可选的萌萌计数器。使用 `Sqlite` 作为后端数据库，使用 `HashMap` 将计数缓存于内存。

<details>
<summary>More theme</summary>

##### asoul

![asoul](https://count.yurzi.net/demo?theme=asoul&format=png)

##### moebooru

![moebooru](https://count.yurzi.net/demo?theme=moebooru&format=png)

##### rule34

![Rule34](https://count.yurzi.net/demo?theme=rule34&format=png)

##### gelbooru

![Gelbooru](https://count.yurzi.net/demo?theme=gelbooru&format=png)

##### e621

![e621](https://count.yurzi.net/demo?theme=e621&format=png)

</details>

## Usage

### Install

You can build this project and deploy the binary on you own server.

### Configuration

After first run, the default config file will be created. See config file for details.

## API & Query

### Route

- `/demo`: can get demo image with query.
- `/:key`: count key and get image with query.
- `/status`: check server status.

### Query

- `theme`: theme you gonnya use (default: `moebooru`), can set default theme in config
- `length`: amount of number to show, will automatically expand if the number is larger than what was set (default: `0`).
- `format`: choose between `svg` and `webp` (default: `svg`).

## Credits

- [replit](https://replit.com/)
- [A-SOUL_Official](https://space.bilibili.com/703007996)
- [moebooru](https://github.com/moebooru/moebooru)
- rule34.xxx NSFW
- gelbooru.com NSFW
- e621.net NSFW
- [Icons8](https://icons8.com/icons/set/star)
- [journey-ad](https://github.com/journey-ad/)

## License

[MIT](LICENSE)
