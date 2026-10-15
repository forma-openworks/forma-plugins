//! «Подсказки терминов»: наведи на term в главе — подсказка из `Глоссарий.md` хранилища:
//! что это, помечен ли он как preview, где его объяснили и где разберут подробнее (колонка
//! «Подробно —» реестра `terms.md` курса, если он есть в служебной папке вроде
//! `.model-kit/` или `.rust-project/`).
//!
//! Формат Глоссария — как в курсах: `- **term** — определение. *preview* → [[глава#раздел]]`.

use forma_plugin_api::{Context, HoverRequest, HoverResult, Plugin, host, register_plugin};

/// Файлы глоссария, которые ищет плагин (в корне хранилища).
const GLOSSARIES: [&str; 2] = ["Глоссарий.md", "Glossary.md"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Term {
    /// Как term написан в глоссарии (с обратными кавычками, если он из code).
    pub title: String,
    /// Варианты написания для поиска в тексте — в нижнем регистре.
    pub keys: Vec<String>,
    pub definition: String,
    pub preview: bool,
    /// Где объяснили: цель `[[ссылки]]`.
    pub link: Option<String>,
    /// Где разберут подробнее (из `terms.md`).
    pub deeper: Option<String>,
}

/// Варианты написания term для поиска: без обратных кавычек и пояснений в скобках, каждый
/// из вариантов через `/` («entry point / `main`»).
pub fn keys_of(name: &str) -> Vec<String> {
    let plain: String = name.replace('`', "");
    let plain = match plain.find(" (") {
        Some(i) => plain[..i].to_owned(),
        None => plain,
    };
    plain.split(" / ").map(|k| k.trim().to_lowercase()).filter(|k| k.chars().count() >= 2).collect()
}

/// Термины из текста глоссария.
pub fn parse_glossary(text: &str) -> Vec<Term> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("- **") else { continue };
        let Some(end) = rest.find("**") else { continue };
        let title = rest[..end].trim().to_owned();
        let mut body = rest[end + 2..].trim_start();
        body = body.strip_prefix('—').or_else(|| body.strip_prefix('-')).unwrap_or(body).trim();
        let (body, link) = match body.rfind("→ [[") {
            Some(i) => {
                let target = body[i + "→ [[".len()..].trim_end().trim_end_matches("]]").to_owned();
                (body[..i].trim(), Some(target))
            }
            None => (body, None),
        };
        let preview = body.contains("*preview*");
        let definition = body.replace("*preview*", "").trim().trim_end_matches('.').trim().to_owned();
        let keys = keys_of(&title);
        if !keys.is_empty() {
            out.push(Term { title, keys, definition, preview, link, deeper: None });
        }
    }
    out
}

/// «Подробно —» из таблиц реестра терминов: term → где разберут подробнее.
pub fn parse_registry(text: &str) -> Vec<(Vec<String>, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let cells: Vec<&str> = line.trim().trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 5 || cells[0] == "Term" || cells[0].starts_with("---") {
            continue;
        }
        let deeper = cells[4];
        if deeper.is_empty() || deeper == "—" || deeper == "-" {
            continue;
        }
        out.push((keys_of(cells[0]), deeper.to_owned()));
    }
    out
}

/// Термин в `text` под символом `offset`: самый длинный, целым словом.
pub fn find<'a>(terms: &'a [Term], text: &str, offset: usize) -> Option<(&'a Term, usize, usize)> {
    let lower: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    // Нижний регистр редких букв меняет длину — тогда номера символов не сойдутся.
    if lower.len() != text.chars().count() || offset >= lower.len() {
        return None;
    }
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut best: Option<(&Term, usize, usize)> = None;
    for term in terms {
        for key in &term.keys {
            let k: Vec<char> = key.chars().collect();
            let n = k.len();
            if n == 0 || n > lower.len() || best.is_some_and(|(_, s, e)| e - s >= n) {
                continue;
            }
            let first = (offset + 1).saturating_sub(n);
            for start in first..=offset.min(lower.len() - n) {
                if lower[start..start + n] == k[..] {
                    let before = start == 0 || !word(lower[start - 1]);
                    let after = start + n == lower.len() || !word(lower[start + n]);
                    if before && after {
                        best = Some((term, start, start + n));
                        break;
                    }
                }
            }
        }
    }
    best
}

/// Текст подсказки (markdown).
pub fn card(term: &Term, english: bool) -> String {
    let mut md = format!("**{}** — {}", term.title, term.definition);
    if term.preview {
        md.push_str(if english { "\n\n*Preview: explained in full later.*" } else { "\n\n*Пока preview: подробно разберут позже.*" });
    }
    // Сначала — где уже объяснили, потом — где разберут глубже: по ходу курса.
    if let Some(link) = &term.link {
        let shown = link.replace('#', " › ");
        md.push_str(&if english { format!("\n\nExplained in: {shown}") } else { format!("\n\nГде объяснили: {shown}") });
    }
    if let Some(d) = &term.deeper {
        md.push_str(&if english { format!("\n\nIn depth: {d}.") } else { format!("\n\nПодробнее — {d}.") });
    }
    md
}

struct Terms {
    terms: Vec<Term>,
    /// Путь глоссария: на нём самом подсказки не нужны.
    glossary: Option<String>,
    english: bool,
}

impl Terms {
    fn load(&mut self) {
        self.terms.clear();
        self.glossary = None;
        for name in GLOSSARIES {
            if let Ok(text) = host::read_file(name) {
                self.terms = parse_glossary(&text);
                self.glossary = Some(name.to_owned());
                break;
            }
        }
        // Реестр терминов курса — в служебной папке с точкой.
        let registry: Vec<(Vec<String>, String)> = host::list_dir("")
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.dir && e.name.starts_with('.'))
            .filter_map(|e| host::read_file(&format!("{}/terms.md", e.name)).ok())
            .flat_map(|t| parse_registry(&t))
            .collect();
        for term in &mut self.terms {
            if let Some((_, deeper)) = registry.iter().find(|(keys, _)| keys.iter().any(|k| term.keys.contains(k))) {
                term.deeper = Some(deeper.clone());
            }
        }
        // Длинные варианты — раньше: «debug build» главнее «build».
        self.terms.sort_by_key(|t| std::cmp::Reverse(t.keys.iter().map(|k| k.chars().count()).max().unwrap_or(0)));
        forma_plugin_api::log(forma_plugin_api::Level::Info, format!("{} terms loaded", self.terms.len()));
    }
}

impl Plugin for Terms {
    fn new(cx: Context) -> Result<Self, String> {
        let mut t = Terms { terms: Vec::new(), glossary: None, english: cx.language.starts_with("en") };
        if cx.root {
            t.load();
        }
        Ok(t)
    }

    fn changed(&mut self, paths: &[String]) {
        // Пустой список — поменяться могло что угодно (хранилище перечитали).
        if paths.is_empty() || paths.iter().any(|p| GLOSSARIES.iter().any(|g| p.ends_with(g)) || p.ends_with("terms.md")) {
            self.load();
        }
    }

    fn hover(&mut self, req: &HoverRequest) -> Option<HoverResult> {
        if self.glossary.as_deref().is_some_and(|g| req.note.replace('\\', "/").ends_with(g)) {
            return None;
        }
        let (term, start, end) = find(&self.terms, &req.text, req.offset as usize)?;
        Some(HoverResult { start: start as u32, end: end as u32, markdown: card(term, self.english) })
    }
}

register_plugin!(Terms);

#[cfg(test)]
mod tests {
    use super::*;

    const GLOSSARY: &str = "## A–C\n\n- **binary crate** — crate с function `main`; из него получается executable. → [[0.1 От source до executable#Package и crate]]\n- **`cargo build`** — собрать executable в папку `target`. → [[0.1 От source до executable#cargo run и папка target]]\n- **attribute** — метка для compiler вида `#[...]`. *preview* → [[0.1 От source до executable#Как читать diagnostics]]\n- **crate** — то, что `rustc` компилирует за один раз. → [[0.1#Package и crate]]\n- **кожурки** — корабли. В курсе их нет. → [[Курс]]\n";

    #[test]
    fn glossary_lines_become_terms() {
        let terms = parse_glossary(GLOSSARY);
        assert_eq!(terms.len(), 5);
        let cargo = &terms[1];
        assert_eq!((cargo.title.as_str(), cargo.keys.as_slice()), ("`cargo build`", &["cargo build".to_owned()][..]));
        assert_eq!(cargo.definition, "собрать executable в папку `target`");
        assert_eq!(cargo.link.as_deref(), Some("0.1 От source до executable#cargo run и папка target"));
        assert!(terms[2].preview && !terms[2].definition.contains("preview"));
        assert_eq!(keys_of("entry point / `main`"), ["entry point", "main"]);
        assert_eq!(keys_of("channel (`stable`, `beta`, `nightly`)"), ["channel"]);
    }

    #[test]
    fn registry_gives_where_it_goes_deeper() {
        let reg = parse_registry("| Term | Статус | Раздел | Зависит от | Подробно — |\n|---|---|---|---|---|\n| machine code | введён | Что такое | — | Module 21 |\n| `rustc` | введён | rustc | compiler | — |\n");
        assert_eq!(reg, vec![(vec!["machine code".to_owned()], "Module 21".to_owned())]);
    }

    #[test]
    fn hover_finds_the_longest_whole_word() {
        let mut terms = parse_glossary(GLOSSARY);
        terms.sort_by_key(|t| std::cmp::Reverse(t.keys.iter().map(|k| k.chars().count()).max().unwrap_or(0)));
        let text = "Каждый binary crate собирается командой cargo build, а кожурки не нужны.";
        let at = |word: &str| text.chars().collect::<String>().find(word).map(|b| text[..b].chars().count()).unwrap();
        let (t, s, e) = find(&terms, text, at("crate") + 2).unwrap();
        assert_eq!((t.title.as_str(), e - s), ("binary crate", "binary crate".chars().count()), "длинный term главнее короткого");
        let (t, ..) = find(&terms, text, at("build")).unwrap();
        assert_eq!(t.title, "`cargo build`");
        let (t, ..) = find(&terms, text, at("кожурки") + 3).unwrap();
        assert_eq!(t.title, "кожурки", "кириллица тоже");
        assert!(find(&terms, "crates и subcrate", 2).is_none(), "только целые слова");
        assert!(find(&terms, text, 0).is_none());
        let md = card(&terms.iter().find(|t| t.preview).unwrap().clone(), false);
        assert!(md.contains("preview") && md.contains("Где объяснили: 0.1 От source до executable › Как читать diagnostics"));
    }
}
