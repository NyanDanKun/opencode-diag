# Y2K Clinical Design System

Принципы дизайна для Rust GUI утилит.

## Философия

- **Минимализм** — никаких лишних элементов
- **Техничность** — моноширинные шрифты, технические метки
- **Двойная тема** — Light/Dark с переключателем
- **Без зависимостей** — один бинарник, работает везде

## Цветовая палитра

### Light Theme
```rust
const LIGHT: Theme = Theme {
    bg: Color32::from_rgb(0xe8, 0xe8, 0xe8),        // #e8e8e8
    window: Color32::from_rgb(0xf7, 0xf7, 0xf7),    // #f7f7f7
    header: Color32::from_rgb(0xff, 0xff, 0xff),    // #ffffff
    panel: Color32::from_rgb(0xff, 0xff, 0xff),     // #ffffff
    text: Color32::from_rgb(0x2a, 0x2a, 0x2a),      // #2a2a2a
    text_dim: Color32::from_rgb(0x88, 0x88, 0x88),  // #888888
    border: Color32::from_rgb(0xa0, 0xa0, 0xa0),    // #a0a0a0
    accent_on: Color32::from_rgb(0x2a, 0x2a, 0x2a), // #2a2a2a
    accent_off: Color32::from_rgb(0xd0, 0xd0, 0xd0),// #d0d0d0
};
```

### Dark Theme
```rust
const DARK: Theme = Theme {
    bg: Color32::from_rgb(0x0f, 0x0f, 0x0f),        // #0f0f0f
    window: Color32::from_rgb(0x1a, 0x1a, 0x1a),    // #1a1a1a
    header: Color32::from_rgb(0x14, 0x14, 0x14),    // #141414
    panel: Color32::from_rgb(0x22, 0x22, 0x22),     // #222222
    text: Color32::from_rgb(0xe0, 0xe0, 0xe0),      // #e0e0e0
    text_dim: Color32::from_rgb(0x5c, 0x5c, 0x5c),  // #5c5c5c
    border: Color32::from_rgb(0x33, 0x33, 0x33),    // #333333
    accent_on: Color32::from_rgb(0x00, 0xbc, 0xd4), // #00bcd4 (cyan glow)
    accent_off: Color32::from_rgb(0x33, 0x33, 0x33),// #333333
};
```

## Типографика

| Элемент | Шрифт | Размер | Стиль |
|---------|-------|--------|-------|
| Заголовок окна | System UI | 14px | Bold, UPPERCASE |
| Section header | Monospace | 10px | `// SECTION NAME` |
| Названия элементов | System UI | 13px | Bold, UPPERCASE |
| Метаданные | Monospace | 9px | `TYPE: VALUE :: INFO` |
| Кнопки | Monospace | 10px | Bold |
| Footer | Monospace | 9px | `SYS.STATUS: TEXT` |

## Компоненты

### Header
```
┌─────────────────────────────────────────────┐
│ ■ APP NAME                          [DARK]  │
├─────────────────────────────────────────────┤
```
- Квадратный индикатор слева (светится в dark mode)
- Название UPPERCASE
- Кнопка темы справа

### Section Header
```
// SECTION NAME
```
- Префикс `//` как комментарий в коде
- Цвет `text_dim`

### Card/Row
```
┃ ITEM NAME                            [ON ]  │
┃ TYPE: LOCAL :: cmd preview here...          │
```
- Вертикальная полоса слева (accent при hover)
- Название UPPERCASE, bold
- Метаданные monospace, dim

### Toggle Button
- ON: `accent_on` фон, белый текст
- OFF: `accent_off` фон, dim текст
- Фиксированная ширина

### Footer
```
SYS.STATUS: MESSAGE                    v0.1.0
```

### Popup Menu / Dropdown
```
// SECTION NAME              [SETTINGS]
                                    │
                    ┌───────────────┴───────────────┐
                    │ // CATEGORY                    │
                    │ ✓ Option One                   │
                    │   Option Two                   │
                    │ ─────────────────────────────  │
                    │ // ANOTHER CATEGORY            │
                    │ ✓ Another Option               │
                    └────────────────────────────────┘
```

**Стилизация popup меню:**

- **Острые углы** — `rounding: 0.0` (без скруглений)
- **Без тени** — `shadow: Shadow::NONE`
- **Фон** — `theme.panel`
- **Рамка** — 1px `theme.border`
- **Шрифт** — Monospace для всего содержимого
- **Заголовки секций** — `// CATEGORY` стиль, `text_dim` цвет, 9px
- **Пункты меню** — Monospace, 10px
- **Чекбоксы** — стандартные egui, но с кастомными цветами
- **Закрытие** — по клику вне области (без кнопки сохранения)
- **Применение** — мгновенное, без подтверждения

**Кнопка вызова:**
```
[SETTINGS]  — текстовая кнопка, не иконка
```
- Monospace, 9px, UPPERCASE
- Фон: `accent_off` (или transparent)
- При активном меню: `accent_on` цвет текста

**Код реализации:**
```rust
egui::Area::new(egui::Id::new("settings_popup"))
    .anchor(egui::Align2::RIGHT_TOP, [-20.0, 40.0])
    .show(ctx, |ui| {
        egui::Frame::none()
            .fill(theme.panel)
            .stroke(egui::Stroke::new(1.0, theme.border))
            .rounding(0.0)  // Острые углы
            .shadow(egui::Shadow::NONE)  // Без тени
            .inner_margin(10.0)
            .show(ui, |ui| {
                // Содержимое меню
            });
    });
```

## Поведение

- **Auto-hide scrollbar** — показывать только когда нужно
- **Hover effects** — accent bar меняет цвет
- **Theme toggle** — мгновенное переключение без перезапуска
- **Auto-refresh** — проверка изменений каждые 2 сек (где применимо)
- **Click outside to close** — popup закрывается при клике вне области

## Структура Rust проекта

```
project-name/
├── Cargo.toml
├── src/
│   ├── main.rs         # Entry point + App struct
│   ├── theme.rs        # Color schemes
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── header.rs   # Header component
│   │   ├── card.rs     # Card/row component
│   │   └── footer.rs   # Footer component
│   └── core/
│       ├── mod.rs
│       └── ...         # Business logic
├── README.md
├── LICENSE
└── .gitignore
```

## Зависимости (минимум)

```toml
[dependencies]
eframe = "0.29"  # egui + native window
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Дополнительно по необходимости:
- `reqwest` — HTTP запросы
- `tokio` — async runtime
- `wmi` — Windows GPU/system info
- `dirs` — пути к конфигурациям
