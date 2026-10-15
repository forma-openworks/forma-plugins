<h1 align="center">Forma Plugins</h1>

<p align="center">
  <b>First-party plugins for FORMA desktop apps.</b><br>
  Themes, note blocks and tools, packed like mods.
</p>

<p align="center">
  <img alt="Status: early development" src="https://img.shields.io/badge/status-early%20development-8a6cf0">
  <img alt="Format: .fpl" src="https://img.shields.io/badge/format-.fpl-e8913a">
  <img alt="Code: WebAssembly" src="https://img.shields.io/badge/code-WebAssembly-654ff0?logo=webassembly&logoColor=white">
</p>

<p align="center">
  <a href="#english">English</a> · <a href="#русский">Русский</a>
</p>

---

## English

### What it is

Plugins for the apps built on [Forma Core](https://github.com/forma-openworks/forma-core): Forma: Holotype, Forma: Instar and [Forma Mycelium](https://github.com/forma-openworks/forma-mycelium). A plugin is a `.fpl` file, a plain zip like a Minecraft mod's `.jar`, with `forma-plugin.toml` at its root. The manifest says which apps the plugin is for, which plugin API it understands, what it depends on, what it may do and what it adds.

The first plugins serve a course on 3D graphics read in Holotype.

| Plugin | What it adds |
|---|---|
| `plugins/studio-theme` | “Studio” themes, dark and light: neutral grey like the Blender viewport, so renders, textures and screenshots keep their true colors |

Coming next: term hints, blocks for the whole 3D pipeline, Myuri for the graphics course, a bridge to Blender, a reference board and a hotkey trainer.

### Installing

Drop a `.fpl` onto the app window, or use Settings ▸ Plugins ▸ Install from file. The app shows what the plugin adds and what it asks for, and installs it only after you agree.

For development: Settings ▸ Plugins ▸ Load a plugin folder. The plugin then runs straight from its folder; press Reload after changes.

### Packing

A plugin without code is just a folder. Pack it with the system `tar` (Windows 10+, macOS):

```bash
tar --format zip -cf dist/studio-theme.fpl -C plugins/studio-theme forma-plugin.toml themes
```

---

## Русский

### Что это

Плагины для программ на [Forma Core](https://github.com/forma-openworks/forma-core): Forma: Holotype, Forma: Instar и [Forma Mycelium](https://github.com/forma-openworks/forma-mycelium). Плагин — файл `.fpl`, обычный zip, как `.jar` у мода Minecraft, с `forma-plugin.toml` в корне. В манифесте сказано, для каких программ плагин, какую версию API плагинов он понимает, от чего зависит, что ему можно и что он добавляет.

Первые плагины — для курса по 3D-графике, который читается в Holotype.

| Плагин | Что добавляет |
|---|---|
| `plugins/studio-theme` | Темы «Студия», тёмная и светлая: нейтральный серый, как во viewport Blender, — цвета рендеров, текстур и скриншотов не искажаются |

Дальше: подсказки терминов, блоки для всего 3D-pipeline, Мьюри для курса графики, мост к Blender, доска референсов и тренажёр горячих клавиш.

### Как поставить

Перетащить `.fpl` в окно программы или «Настройки ▸ Плагины ▸ Установить из файла». Программа покажет, что плагин добавляет и что просит, и поставит его только после согласия.

Для разработки: «Настройки ▸ Плагины ▸ Подключить папку плагина». Плагин работает прямо из своей папки; после правок — «Обновить».

### Как упаковать

Плагин без code — просто папка. Упаковать её можно системным `tar` (Windows 10+, macOS):

```bash
tar --format zip -cf dist/studio-theme.fpl -C plugins/studio-theme forma-plugin.toml themes
```
