//! «Пример плагина» — образец для авторов плагинов Forma (API 1.2). Одна панель показывает:
//!
//! - **программу в фоне** (`host::spawn`): `ping` шесть раз, строки вывода приходят по одной в
//!   [`Plugin::job_event`], «Остановить» (`host::cancel`) убивает всё, что запущено;
//! - **связанные папки** хранилища-курса (`host::linked_dirs`, право `linked:read`);
//! - **правку заметки через программу**: отметить первый пункт «Практики» текущей заметки
//!   (`host::toggle_task`) и дописать строку в `Вопросы/Пример.md` (`host::append_to_note`) —
//!   несохранённые правки открытой заметки при этом не теряются.
//!
//! Права в манифесте: `process = ["ping"]`, `files = ["vault:read", "vault:write",
//! "linked:read"]`.

use forma_plugin_api::ui::{self, Element, Event, EventKind, Tone};
use forma_plugin_api::{CommandContext, Context, JobEvent, JobStream, PanelContext, Plugin, host, register_plugin};

const PANEL: &str = "demo";
/// Больше строк вывода не держим: панель — не журнал.
const MAX_LINES: usize = 200;

#[derive(Default)]
struct Example {
    english: bool,
    /// Текущая задача и идёт ли она.
    job: Option<u32>,
    running: bool,
    /// Задачу остановили кнопкой (а не сама завершилась).
    stopped: bool,
    exit: Option<Option<i32>>,
    lines: Vec<String>,
    /// Заметка, для которой нарисована панель.
    note: String,
    /// Что получилось у последней кнопки.
    message: String,
}

/// Жирное начало первого неотмеченного пункта раздела «Практика».
pub fn first_open_task(text: &str) -> Option<String> {
    let mut inside = false;
    for line in text.lines() {
        if line.trim_end() == "## Практика" {
            inside = true;
            continue;
        }
        if inside && line.starts_with("## ") {
            break;
        }
        if inside && let Some(rest) = line.strip_prefix("- [ ] **") {
            return rest.split_once("**").map(|(t, _)| t.trim().to_owned());
        }
    }
    None
}

impl Example {
    fn tr<'a>(&self, ru: &'a str, en: &'a str) -> &'a str {
        if self.english { en } else { ru }
    }

    fn button(&self, id: &str, text: &str, enabled: bool, primary: bool) -> Element {
        Element::Button { id: id.into(), text: text.into(), icon: None, primary, enabled, tooltip: String::new(), command: None, args: Default::default() }
    }

    fn panel(&self) -> Element {
        let mut c = vec![
            ui::heading(self.tr("Программа в фоне", "A background program")),
            ui::hint(self.tr("ping шесть раз: строки приходят по одной, «Остановить» убивает всё, что запущено.", "ping six times: lines arrive one by one, Stop kills everything it started.")),
            ui::row([self.button("run", self.tr("Запустить ping", "Run ping"), !self.running, true), self.button("stop", self.tr("Остановить", "Stop"), self.running, false)]),
        ];
        let status = match (self.running, self.exit) {
            (true, _) => Some((self.tr("идёт…", "running…").to_owned(), Tone::Accent)),
            (false, Some(_)) if self.stopped => Some((self.tr("остановлена", "stopped").to_owned(), Tone::Muted)),
            (false, Some(Some(code))) => Some((format!("{} {code}", self.tr("завершилась, код", "finished, code")), if code == 0 { Tone::Success } else { Tone::Warning })),
            (false, Some(None)) => Some((self.tr("остановлена", "stopped").to_owned(), Tone::Muted)),
            _ => None,
        };
        if let Some((text, tone)) = status {
            c.push(Element::Label { text, icon: None, tone, size: ui::Size::Small, bold: true });
        }
        if !self.lines.is_empty() {
            c.push(ui::code(self.lines.join("\n"), ""));
        }
        c.push(Element::Separator);
        c.push(ui::heading(self.tr("Связанные папки", "Linked folders")));
        let dirs = host::linked_dirs();
        if dirs.is_empty() {
            c.push(ui::hint(self.tr("Нет: хранилище — не курс (или у плагина нет права linked:read).", "None: the vault is not a course (or the plugin lacks linked:read).")));
        } else {
            c.push(ui::list("dirs", dirs.iter().map(|d| ui::Item { icon: Some("folder".into()), ..ui::item(d.clone(), d.clone()) }), None));
        }
        c.push(Element::Separator);
        c.push(ui::heading(self.tr("Заметка", "Note")));
        let note = if self.note.is_empty() { self.tr("нет открытой заметки", "no note is open").to_owned() } else { self.note.clone() };
        c.push(ui::hint(note));
        c.push(ui::row([
            self.button("tick", self.tr("Отметить первый пункт «Практики»", "Tick the first practice task"), !self.note.is_empty(), false),
            self.button("append", self.tr("Дописать в «Вопросы/Пример.md»", "Append to Вопросы/Пример.md"), true, false),
        ]));
        if !self.message.is_empty() {
            c.push(ui::label(self.message.clone()));
        }
        ui::column(c)
    }
}

impl Plugin for Example {
    fn new(cx: Context) -> Result<Self, String> {
        Ok(Example { english: cx.language.starts_with("en"), ..Default::default() })
    }

    fn command(&mut self, _id: &str, _cx: &CommandContext) -> Result<String, String> {
        host::open_panel(PANEL);
        Ok(String::new())
    }

    fn render_panel(&mut self, _panel: &str, cx: &PanelContext) -> Result<Element, String> {
        self.note = cx.note.clone();
        Ok(self.panel())
    }

    fn panel_event(&mut self, _panel: &str, e: &Event) {
        if e.kind != EventKind::Click {
            return;
        }
        match e.id.as_str() {
            "run" => match host::spawn("ping", &["-n".into(), "6".into(), "127.0.0.1".into()], "") {
                Ok(job) => {
                    (self.job, self.running, self.stopped, self.exit) = (Some(job), true, false, None);
                    self.lines.clear();
                }
                Err(err) => self.message = err,
            },
            "stop" => {
                if let Some(job) = self.job {
                    host::cancel(job);
                    self.stopped = true;
                }
            }
            "tick" => {
                self.message = match host::read_file(&self.note).map(|t| first_open_task(&t)) {
                    Ok(Some(title)) => match host::toggle_task(&self.note, &title) {
                        Ok(()) => format!("{} «{title}»", self.tr("Отмечен", "Ticked")),
                        Err(e) => e,
                    },
                    Ok(None) => self.tr("В «Практике» нет неотмеченных пунктов.", "No open practice tasks.").to_owned(),
                    Err(e) => e,
                };
            }
            "append" => {
                let line = format!("- {}", self.tr("Запись из «Примера плагина».", "A line from the example plugin."));
                self.message = match host::append_to_note("Вопросы/Пример.md", &line) {
                    Ok(()) => self.tr("Дописано в «Вопросы/Пример.md».", "Appended to Вопросы/Пример.md.").to_owned(),
                    Err(e) => e,
                };
            }
            _ => {}
        }
    }

    fn job_event(&mut self, e: &JobEvent) {
        if Some(e.job) != self.job {
            return;
        }
        match e.kind {
            JobStream::Stdout | JobStream::Stderr => {
                if !e.text.trim().is_empty() {
                    self.lines.push(e.text.clone());
                    if self.lines.len() > MAX_LINES {
                        self.lines.remove(0);
                    }
                }
            }
            JobStream::Exit => {
                self.running = false;
                self.exit = Some(e.code);
            }
        }
        host::refresh_panel(PANEL);
    }
}

register_plugin!(Example);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_first_open_practice_task() {
        let text = "## Практика\n\n- [x] **Готово.** уже\n- [ ] **Свой package.** Создай\n\n## Дальше\n- [ ] **Не то.**\n";
        assert_eq!(first_open_task(text).as_deref(), Some("Свой package."));
        assert!(first_open_task("## Практика\n- [x] **Всё.**\n").is_none());
    }
}
