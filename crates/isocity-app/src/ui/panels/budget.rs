//! The budget panel: per-department funding.

use crate::ui::game_view::{slider, GameView, SliderKind};
use crate::ui::widgets::money;
use crate::theme;
use gpui::{div, prelude::*, px, AnyElement, Context};
use isocity_core::BudgetKey;

pub(super) fn budget_body(this: &GameView, cx: &mut Context<GameView>) -> AnyElement {
    let w = this.world.borrow();
    let st = w.state();
    let (income, expenses) = (st.stats.income, st.stats.expenses);
    let cats: Vec<(BudgetKey, String, f32, i64)> = BudgetKey::ALL
        .iter()
        .map(|k| {
            let c = st.budget.get(*k);
            (*k, c.name.clone(), c.funding, c.cost)
        })
        .collect();
    drop(w);
    let net = income - expenses;
    let col = |label: &str, value: String, color: gpui::Hsla| {
        div()
            .flex_1()
            .child(div().text_size(px(12.0)).text_color(theme::muted_foreground()).mb(px(4.0)).child(label.to_string()))
            .child(div().text_color(color).child(value))
    };
    let bounds = this.slider_bounds.clone();
    div()
        .flex()
        .flex_col()
        .gap(px(24.0))
        .child(
            div()
                .flex()
                .gap(px(16.0))
                .pb(px(16.0))
                .border_b_1()
                .border_color(theme::border())
                .child(col("Income", format!("{}/mo", money(income)), theme::green_400()))
                .child(col("Expenses", format!("{}/mo", money(expenses)), theme::red_400()))
                .child(col("Net", format!("{}/mo", money(net)), if net >= 0 { theme::green_400() } else { theme::red_400() })),
        )
        .child(div().flex().flex_col().gap(px(16.0)).children(cats.into_iter().map(|(k, name, funding, cost)| {
            div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .child(div().w(px(112.0)).text_size(px(14.0)).child(name))
                .child(div().flex_1().child(slider(SliderKind::Budget(k), funding / 100.0, 220.0, bounds.clone(), cx)))
                .child(div().w(px(48.0)).text_size(px(14.0)).child(format!("{}%", funding as i32)))
                .child(div().w(px(72.0)).text_size(px(12.0)).text_color(theme::muted_foreground()).child(format!("{}/mo", money((cost as f32 * funding / 100.0) as i64))))
                .into_any_element()
        })))
        .into_any_element()
}
