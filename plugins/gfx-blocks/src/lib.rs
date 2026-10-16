//! «Блоки графики» — блоки заметок для курса по компьютерной графике (Holotype):
//!
//! - ` ```pipeline ` — карта этапов работы: что готово, что идёт, что впереди;
//! - ` ```nodes ` — граф нод, как в редакторе нод Blender;
//! - ` ```compare ` — «до и после» двух картинок с ползунком;
//! - ` ```budget ` — бюджет полигонов: из чего складываются треугольники;
//! - ` ```material ` — карточка PBR-материала с шаром;
//! - ` ```storyboard ` — раскадровка карточками;
//! - ` ```gltf ` — модель glTF, которую можно вращать, и сведения о ней;
//! - ` ```uv ` — UV-раскладка модели.
//!
//! Как писать каждый блок — в начале его модуля. Рисует программа (дерево элементов и
//! холст из `forma-plugin-ui`), плагин только считает и раскладывает.

mod budget;
mod material;
mod model;
mod nodes;
mod pipeline;
mod storyboard;
mod util;
mod views;

use forma_plugin_api::ui::Element;
use forma_plugin_api::{Context, Plugin, register_plugin};

use util::{Cx, key_value};

struct Blocks {
    english: bool,
}

/// ` ```compare `: `before:`, `after:` и подписи (`labels: До | После`).
fn compare(source: &str, cx: &Cx) -> Result<Element, String> {
    let (mut before, mut after, mut labels) = (String::new(), String::new(), (cx.tr("До", "Before").to_owned(), cx.tr("После", "After").to_owned()));
    for line in source.lines() {
        let Some((k, v)) = key_value(line) else { continue };
        let v = v.trim_start_matches("![[").trim_end_matches("]]").to_owned();
        match k.as_str() {
            "before" | "до" => before = v,
            "after" | "после" => after = v,
            "labels" | "подписи" => {
                if let Some((a, b)) = v.split_once('|') {
                    labels = (a.trim().to_owned(), b.trim().to_owned());
                }
            }
            _ => {}
        }
    }
    if before.is_empty() || after.is_empty() {
        return Err(cx.tr("нужны две картинки: «before: …» и «after: …»", "two images are needed: “before: …” and “after: …”").into());
    }
    Ok(Element::Compare { before, after, before_label: labels.0, after_label: labels.1 })
}

impl Plugin for Blocks {
    fn new(cx: Context) -> Result<Self, String> {
        Ok(Blocks { english: cx.language.starts_with("en") })
    }

    fn render_block(&mut self, lang: &str, source: &str, context: &str) -> Result<Element, String> {
        let cx = Cx::new(context, self.english);
        match lang.to_lowercase().as_str() {
            "pipeline" => pipeline::render(source, &cx),
            "nodes" => nodes::render(source, &cx),
            "compare" => compare(source, &cx),
            "budget" => budget::render(source, &cx),
            "material" => material::render(source, &cx),
            "storyboard" => storyboard::render(source, &cx),
            "gltf" => views::render_gltf(source, &cx, &model::read_vault),
            "uv" => views::render_uv(source, &cx, &model::read_vault),
            other => Err(format!("unknown block {other}")),
        }
    }
}

register_plugin!(Blocks);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_needs_both_images_and_reads_labels() {
        let cx = Cx::default();
        let e = compare("before: ![[r/no_ao.png]]\nafter: r/ao.png\nlabels: Без AO | С AO", &cx).unwrap();
        assert_eq!(e, Element::Compare { before: "r/no_ao.png".into(), after: "r/ao.png".into(), before_label: "Без AO".into(), after_label: "С AO".into() });
        assert!(compare("before: a.png", &cx).is_err());
    }

    #[test]
    fn every_block_answers_or_explains() {
        let mut b = Blocks { english: false };
        for lang in ["pipeline", "nodes", "budget", "material", "storyboard", "gltf", "uv", "compare"] {
            // Пустой блок: либо рисуется (материал по умолчанию), либо объясняет, чего ждёт.
            match b.render_block(lang, "", "{}") {
                Ok(_) => assert_eq!(lang, "material"),
                Err(e) => assert!(!e.is_empty(), "{lang}"),
            }
        }
        assert!(b.render_block("hologram", "", "{}").is_err());
    }
}
