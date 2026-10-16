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
| `plugins/terms` | Term hints: hover a term in a chapter and get a card from the vault's `Glossary.md` (`Глоссарий.md`): what it means, whether it is only a preview yet, where it was explained and where the course goes deeper. Also a “Glossary” panel with search, “Find in glossary” on selected text and a ```` ```terms ```` block with the cards of a chapter’s new terms. The first plugin with code |
| `plugins/gfx-blocks` | Graphics blocks for notes: a pipeline map, a Blender-like node graph, before/after, a polygon budget, a PBR material card, a storyboard, a glTF viewer and a UV layout |
| `plugins/example` | An example for plugin authors (API 1.2), for Holotype and Instar: a panel that runs `ping` in the background with output line by line and Stop, lists the course's linked folders, ticks a practice task and appends to a note; a hover on `TODO` and a “what is selected” item in the selected-text menu |

Coming next: Myuri for the graphics course, a bridge to Blender, a reference board and a hotkey trainer.

### Graphics blocks at a glance

Each block is a fenced code block in a note; the toggle in its header shows the source.

| Block | Write | Shows |
|---|---|---|
| ```` ```pipeline ```` | `Modeling: now — fuselage` per line (`done`, `now`, `next`, `skip`) | stages as a flow, what is done and what is next |
| ```` ```nodes ```` | `Principled BSDF [shader]`, sockets `  > Roughness: float = 0.5` / `  < BSDF: shader`, links `A.Out -> B.In` | a node graph that looks like Blender's (colors from the Blender 5.3 sources), pan and Ctrl+wheel zoom |
| ```` ```compare ```` | `before:`, `after:`, `labels: Before \| After` | two vault images with a draggable divider |
| ```` ```budget ```` | `Budget: 8000 tris`, then `Fuselage: 4200` | parts of a triangle budget as one bar with the budget mark and a table |
| ```` ```material ```` | `base color: #6b7350`, `metallic`, `roughness`, `emission`, `normal map: …` | a PBR ball (GGX, like Principled BSDF) with the parameters and maps |
| ```` ```storyboard ```` | `## Takeoff · 4s`, an image path, `Camera: …`, a description | shots as cards and the running time |
| ```` ```gltf ```` | `model: models/plane.glb`, `wireframe: true` | the model to orbit, with triangles, vertices, materials, UVs and size |
| ```` ```uv ```` | `model: models/plane.glb`, optional `mesh: Fuselage` | the UV layout like Blender's UV editor: islands, used share of 0–1, points outside |

### Installing

Drop a `.fpl` onto the app window, or use Settings ▸ Plugins ▸ Install from file. The app shows what the plugin adds and what it asks for, and installs it only after you agree.

For development: Settings ▸ Plugins ▸ Load a plugin folder. The plugin then runs straight from its folder; press Reload after changes.

### Packing

A plugin without code is just a folder. Pack it with the system `tar` (Windows 10+, macOS):

```bash
tar --format zip -cf dist/studio-theme.fpl -C plugins/studio-theme forma-plugin.toml themes
```

### Plugins with code

Plugin code is a WebAssembly component (`wasm32-wasip2`) written in Rust against the SDK `forma-plugin-api` from Forma Core: implement the `Plugin` trait and call `register_plugin!`. The app runs it in a sandbox (wasmtime). The plugin sees only the files its `[permissions]` allow, and each call is limited in memory and time.

API 1.2 adds background programs (`spawn`, `cancel`, `write-stdin`; output comes to `Plugin::job_event` line by line, and the app kills the whole process tree on Stop, unload and exit), note edits through the app (`toggle-task`, `append-to-note`, `append-to-section`: unsaved edits of an open note are kept) and the course's linked folders (`linked-dirs`, permission `linked:read`). Plugins built for 1.0 and 1.1 keep working. `plugins/example` shows all of it.

In Forma: Instar (`apps = ["instar"]`) plugin panels are area editors next to the code editor and in View, commands are in Find Action and the keymap, `selection` and `note` menu items are in the editor's right-click menu, and hovers show under the language plugin's hover. The file a plugin gets is a path from the project root, like a note in Holotype. Note edits are Holotype's: Instar answers them with a notice.

```bash
rustup target add wasm32-wasip2
cargo xtask build terms   # into target/fpl/terms — load this folder in developer mode
cargo xtask pack terms    # the same, packed into dist/terms.fpl
cargo xtask pack all      # every plugin in plugins/
```

---

## Русский

### Что это

Плагины для программ на [Forma Core](https://github.com/forma-openworks/forma-core): Forma: Holotype, Forma: Instar и [Forma Mycelium](https://github.com/forma-openworks/forma-mycelium). Плагин — файл `.fpl`, обычный zip, как `.jar` у мода Minecraft, с `forma-plugin.toml` в корне. В манифесте сказано, для каких программ плагин, какую версию API плагинов он понимает, от чего зависит, что ему можно и что он добавляет.

Первые плагины — для курса по 3D-графике, который читается в Holotype.

| Плагин | Что добавляет |
|---|---|
| `plugins/studio-theme` | Темы «Студия», тёмная и светлая: нейтральный серый, как во viewport Blender, — цвета рендеров, текстур и скриншотов не искажаются |
| `plugins/terms` | Подсказки терминов: наведи на term в главе — карточка из `Глоссарий.md` хранилища: что это, пока ли это только preview, где его объяснили и где курс разберёт его подробнее. Ещё панель «Глоссарий» с поиском, «Найти в глоссарии» у выделенного текста и блок ```` ```terms ```` с карточками новых терминов главы. Первый плагин с code |
| `plugins/gfx-blocks` | Блоки графики для заметок: карта pipeline, граф нод как в Blender, до и после, бюджет полигонов, карточка PBR-материала, storyboard, просмотр glTF и UV-раскладка |
| `plugins/example` | Пример для авторов плагинов (API 1.2), для Holotype и Instar: панель запускает `ping` в фоне с выводом по строкам и «Остановить», показывает связанные папки курса, отмечает пункт практики и дописывает строку в заметку; подсказка при наведении на `TODO` и пункт «что выделено» в меню выделенного текста |

Дальше: Мьюри для курса графики, мост к Blender, доска референсов и тренажёр горячих клавиш.

### Блоки графики коротко

Каждый блок — блок кода в заметке; переключатель в его шапке показывает исходник.

| Блок | Как писать | Что показывает |
|---|---|---|
| ```` ```pipeline ```` | по этапу в строке: `Modeling: now — фюзеляж` (`done`, `now`, `next`, `skip`) | этапы цепочкой: что готово и что дальше |
| ```` ```nodes ```` | `Principled BSDF [shader]`, сокеты `  > Roughness: float = 0.5` / `  < BSDF: shader`, связи `A.Out -> B.In` | граф нод, как в Blender (цвета из исходников Blender 5.3), двигается и приближается Ctrl и колесом |
| ```` ```compare ```` | `before:`, `after:`, `labels: До \| После` | две картинки хранилища с ползунком |
| ```` ```budget ```` | `Бюджет: 8000 tris`, потом `Фюзеляж: 4200` | части бюджета треугольников одной полосой с отметкой бюджета и таблицей |
| ```` ```material ```` | `base color: #6b7350`, `metallic`, `roughness`, `emission`, `normal map: …` | шар PBR (GGX, как Principled BSDF) с параметрами и картами |
| ```` ```storyboard ```` | `## Взлёт · 4s`, путь к картинке, `Камера: …`, описание | кадры карточками и общее время |
| ```` ```gltf ```` | `model: models/plane.glb`, `wireframe: true` | модель, которую можно вращать, и её треугольники, вершины, материалы, UV, размеры |
| ```` ```uv ```` | `model: models/plane.glb`, можно `mesh: Fuselage` | UV-раскладка, как в UV-редакторе Blender: острова, занятая доля 0–1, точки за квадратом |

### Как поставить

Перетащить `.fpl` в окно программы или «Настройки ▸ Плагины ▸ Установить из файла». Программа покажет, что плагин добавляет и что просит, и поставит его только после согласия.

Для разработки: «Настройки ▸ Плагины ▸ Подключить папку плагина». Плагин работает прямо из своей папки; после правок — «Обновить».

### Как упаковать

Плагин без code — просто папка. Упаковать её можно системным `tar` (Windows 10+, macOS):

```bash
tar --format zip -cf dist/studio-theme.fpl -C plugins/studio-theme forma-plugin.toml themes
```

### Плагины с code

Code плагина — компонент WebAssembly (`wasm32-wasip2`) на Rust с SDK `forma-plugin-api` из Forma Core: реализовать trait `Plugin` и вызвать `register_plugin!`. Программа запускает его в песочнице (wasmtime). Плагин видит только файлы, разрешённые в `[permissions]`, а каждый вызов ограничен по памяти и времени.

API 1.2 добавляет программы в фоне (`spawn`, `cancel`, `write-stdin`; вывод приходит в `Plugin::job_event` по строкам, а «Остановить», выгрузка плагина и выход из программы убивают всё дерево процессов), правки заметок через программу (`toggle-task`, `append-to-note`, `append-to-section`: несохранённые правки открытой заметки не теряются) и связанные папки курса (`linked-dirs`, право `linked:read`). Плагины под 1.0 и 1.1 работают как раньше. Всё это показывает `plugins/example`.

В Forma: Instar (`apps = ["instar"]`) панели плагинов — виды областей рядом с редактором кода и в «Виде», команды — в «Найти действие» и раскладке клавиш, пункты меню `selection` и `note` — в меню правого щелчка редактора, подсказки — под подсказкой языкового плагина. Файл для плагина — путь от корня проекта, как заметка в Holotype. Правки заметок — дело Holotype: Instar отвечает на них уведомлением.

```bash
rustup target add wasm32-wasip2
cargo xtask build terms   # в target/fpl/terms — эту папку можно подключить в режиме разработчика
cargo xtask pack terms    # то же и упаковать в dist/terms.fpl
cargo xtask pack all      # все плагины из plugins/
```
