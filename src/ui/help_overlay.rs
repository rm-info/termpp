use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Background, Color, Element, Length};

use crate::config::Keybindings;
use crate::ui::theme::Theme as AppTheme;

pub fn help_overlay<Message: Clone + 'static>(
    keybindings: &Keybindings,
    on_close: Message,
) -> Element<'static, Message> {
    // Clone all strings immediately — no borrows from keybindings may escape
    let shortcuts: Vec<(&'static str, String)> = vec![
        ("Scinder horizontal",  keybindings.split_horizontal.clone()),
        ("Scinder vertical",    keybindings.split_vertical.clone()),
        ("Pane suivant",        keybindings.pane_next.clone()),
        ("Pane précédent",      keybindings.pane_prev.clone()),
        ("Nouveau pane",        keybindings.new_pane.clone()),
        ("Renommer le pane",    keybindings.rename_pane.clone()),
        ("Fermer le pane",      keybindings.close_pane.clone()),
        ("Aide",                "F1".to_string()),
    ];

    let close_msg = on_close.clone();
    let close_btn: Element<'static, Message> = mouse_area(
        text("×").color(AppTheme::TEXT_DIM).size(14)
    )
    .on_press(close_msg)
    .into();

    let header: Element<'static, Message> = row![
        text("Raccourcis")
            .color(AppTheme::TEXT_PRIMARY)
            .size(15)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::DEFAULT }),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(iced::Alignment::Center)
    .into();

    let separator: Element<'static, Message> = container(Space::new().height(1))
        .width(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::PANE_BORDER)),
            ..Default::default()
        })
        .into();

    let mut rows: Vec<Element<'static, Message>> = vec![header, separator];

    for (label, key) in shortcuts {
        let badge: Element<'static, Message> = container(
            text(key).color(AppTheme::TEXT_PRIMARY).size(12)
        )
        .padding([2, 6])
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(
                Color { r: 0.10, g: 0.10, b: 0.15, a: 1.0 }
            )),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into();

        let shortcut_row: Element<'static, Message> = row![
            text(label).color(AppTheme::TEXT_DIM).size(12),
            Space::new().width(Length::Fill),
            badge,
        ]
        .align_y(iced::Alignment::Center)
        .into();

        rows.push(shortcut_row);
    }

    let card_content = column(rows).spacing(8);

    let card: Element<'static, Message> = container(card_content)
        .width(320)
        .padding(20)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::PANE_BG)),
            border: iced::Border {
                color: AppTheme::PANE_BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into();

    // Backdrop: full-screen semi-transparent overlay, card centered
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::Alignment::Center)
        .align_y(iced::Alignment::Center)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(
                Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 }
            )),
            ..Default::default()
        })
        .into()
}
