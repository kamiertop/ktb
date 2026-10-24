use super::logic::{self, Unit};
use crate::ui::button::{Button, ButtonVariant};
use crate::ui::{panels::eyebrow, theme};
use chrono::{DateTime, Datelike, Utc};
use gpui_kit::component::{
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    input::{Input, InputState},
    tab::{Tab, TabBar},
    time_field::TimePrecision,
};
use gpui_kit::*;

pub struct Page {
    stamp: Entity<InputState>,
    date: Entity<InputState>,
    zone: Entity<InputState>,
    format: Entity<InputState>,
    picker: Entity<DatePickerState>,
    unit: Unit,
    result: Vec<(String, String)>,
    last: Option<DateTime<Utc>>,
    error: String,
    _subscriptions: Vec<Subscription>,
}
impl Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let picker =
            cx.new(|cx| DatePickerState::new(window, cx).time_precision(TimePrecision::Second));
        let subscription = cx.subscribe_in(
            &picker,
            window,
            |this, _, event: &DatePickerEvent, window, cx| {
                let DatePickerEvent::Change(value) = event;
                let Some(date) = value.start() else {
                    return;
                };
                this.date.update(cx, |s, cx| {
                    s.set_value(date.format("%Y-%m-%d %H:%M:%S").to_string(), window, cx)
                });
                this.error.clear();
                cx.notify();
            },
        );
        Self {
            stamp: cx.new(|cx| InputState::new(window, cx).placeholder("Unix 时间戳")),
            date: cx
                .new(|cx| InputState::new(window, cx).placeholder("日期时间或带时区的 ISO 8601")),
            zone: cx.new(|cx| InputState::new(window, cx).default_value("+08:00")),
            format: cx
                .new(|cx| InputState::new(window, cx).default_value("%Y-%m-%d %H:%M:%S%.3f %:z")),
            picker,
            unit: Unit::Seconds,
            result: Vec::new(),
            last: None,
            error: String::new(),
            _subscriptions: vec![subscription],
        }
    }
    fn apply(
        &mut self,
        date: DateTime<Utc>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let zone = logic::offset(&self.zone.read(cx).value())?;
        let local = date.with_timezone(&zone);
        if !(1..=9999).contains(&local.year()) || !(1..=9999).contains(&date.year()) {
            return Err("日期超出显示范围（0001–9999 年）".into());
        }
        let custom = logic::format_custom(date, zone, &self.format.read(cx).value())?;
        let mut rows = vec![
            (
                "日期时间".into(),
                local.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            ),
            ("ISO 8601 / RFC 3339".into(), local.to_rfc3339()),
            ("RFC 2822".into(), local.to_rfc2822()),
            ("UTC".into(), date.to_rfc3339()),
        ];
        rows.push(("自定义格式".into(), custom));
        for unit in Unit::ALL {
            rows.push((
                format!("Unix / {}", unit.label()),
                logic::timestamp(date, unit),
            ));
        }
        self.result = rows;
        self.last = Some(date);
        self.stamp.update(cx, |s, cx| {
            s.set_value(logic::timestamp(date, self.unit), window, cx)
        });
        self.date.update(cx, |s, cx| {
            s.set_value(local.format("%Y-%m-%d %H:%M:%S%.f").to_string(), window, cx)
        });
        self.picker
            .update(cx, |s, cx| s.set_date_time(local.naive_local(), window, cx));
        Ok(())
    }
    fn run(&mut self, from_date: bool, window: &mut Window, cx: &mut Context<Self>) {
        let result = if from_date {
            logic::offset(&self.zone.read(cx).value())
                .and_then(|zone| logic::parse_date(&self.date.read(cx).value(), zone))
        } else {
            logic::from_timestamp(&self.stamp.read(cx).value(), self.unit)
        };
        let result = result.and_then(|date| self.apply(date, window, cx));
        self.error = result.err().unwrap_or_default();
        if !self.error.is_empty() {
            self.result.clear();
            self.last = None;
        }
        cx.notify();
    }
}
impl Page {
    fn conversion_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_4()
            .child(div().flex().items_start().gap_4()
                .child(div().flex().flex_col().flex_1().min_w_0().gap_3().pr_4()
                    .border_r_1().border_color(rgb(theme::LINE))
                    .child(eyebrow("时间戳 → 日期"))
                    .child(div().h(px(36.)).flex().items_center()
                        .child(TabBar::new("timestamp-unit").segmented()
                            .selected_index(Unit::ALL.iter().position(|v| *v == self.unit).unwrap_or(0))
                            .children(Unit::ALL.into_iter().map(|unit| Tab::new().label(unit.label())))
                            .on_click(cx.listener(|this, index: &usize, _, cx| { this.unit = Unit::ALL[*index]; cx.notify(); }))))
                    .child(Input::new(&self.stamp).h(px(36.)).text_size(px(theme::EDITOR_SIZE)).font_weight(theme::FONT_WEIGHT))
                    .child(div().flex().child(Button::new("from-stamp", "转为日期").primary()
                        .on_click(cx.listener(|this, _, w, cx| this.run(false, w, cx))))))
                .child(div().flex().flex_col().flex_1().min_w_0().gap_3()
                    .child(eyebrow("日期 → 时间戳"))
                    .child(DatePicker::new(&self.picker).placeholder("选择日期时间").w_full().h(px(36.)))
                    .child(Input::new(&self.date).h(px(36.)).text_size(px(theme::EDITOR_SIZE)).font_weight(theme::FONT_WEIGHT))
                    .child(div().flex().child(Button::new("from-date", "转为时间戳").primary()
                        .on_click(cx.listener(|this, _, w, cx| this.run(true, w, cx)))))))
            .child(div().flex().flex_col().gap_2().pt_3().border_t_1().border_color(rgb(theme::LINE))
                .child(eyebrow("自定义输出格式"))
                .child(div().flex().items_center().gap_3()
                    .child(div().flex_1().min_w_0().child(Input::new(&self.format)))
                    .child(Button::new("format", "应用格式").on_click(cx.listener(|this, _, w, cx| {
                        this.error = if let Some(date) = this.last { this.apply(date, w, cx).err().unwrap_or_default() } else { "请先转换一个时间".into() }; cx.notify();
                    }))))
                .child(div().text_size(px(theme::LABEL_SIZE)).text_color(theme::muted()).child("%Y 年 · %m 月 · %d 日 · %H 时 · %M 分 · %S 秒 · %.3f 毫秒 · %:z 时区")))
    }
    fn results(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .pt_3()
            .border_t_1()
            .border_color(rgb(theme::LINE))
            .child(eyebrow("转换结果"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(theme::LINE))
                    .bg(rgb(theme::SURFACE))
                    .children(self.result.iter().enumerate().map(|(i, (label, value))| {
                        let copy = value.clone();
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_3()
                            .py_1()
                            .border_b_1()
                            .border_color(rgb(theme::LINE))
                            .child(
                                div()
                                    .w(px(170.))
                                    .flex_shrink_0()
                                    .text_size(px(theme::LABEL_SIZE))
                                    .text_color(theme::muted())
                                    .child(label.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(theme::EDITOR_SIZE))
                                    .child(value.clone()),
                            )
                            .child(
                                Button::new(("copy", i), "复制")
                                    .variant(ButtonVariant::Ghost)
                                    .on_click(move |_, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            copy.clone(),
                                        ))
                                    }),
                            )
                    })),
            )
    }
}
impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(eyebrow("时区"))
                            .child(div().w(px(112.)).child(Input::new(&self.zone)))
                            .children(
                                [("utc", "UTC", "+00:00"), ("utc8", "UTC+8", "+08:00")]
                                    .into_iter()
                                    .map(|(id, label, value)| {
                                        Button::new(id, label)
                                            .variant(ButtonVariant::Ghost)
                                            .on_click(cx.listener(move |this, _, w, cx| {
                                                this.zone
                                                    .update(cx, |s, cx| s.set_value(value, w, cx));
                                                if let Some(date) = this.last {
                                                    this.error = this
                                                        .apply(date, w, cx)
                                                        .err()
                                                        .unwrap_or_default();
                                                }
                                                cx.notify();
                                            }))
                                    }),
                            ),
                    )
                    .child(Button::new("now", "当前时间").on_click(cx.listener(
                        |this, _, w, cx| {
                            this.error = this.apply(Utc::now(), w, cx).err().unwrap_or_default();
                            cx.notify();
                        },
                    ))),
            )
            .child(self.conversion_form(cx))
            .children((!self.error.is_empty()).then(|| {
                div()
                    .text_color(rgb(theme::ERROR))
                    .text_size(px(theme::LABEL_SIZE))
                    .child(self.error.clone())
            }))
            .children((!self.result.is_empty()).then(|| self.results()))
    }
}
