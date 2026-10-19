# 01. Фундамент: workspace, Trunk, первая 3D-сцена

**ID:** T01
**Зависимости:** нет
**Артефакты:** `Cargo.toml`, `crates/lift-core`, `crates/lift-app`, `web/`
**Сложность:** низкая
**Оценка:** 0.5–1 день

## 1. Цель

Получить запускаемый в браузере WASM-модуль, в котором видна пустая кабина лифта.
Никакой физики и UI на этом этапе не требуется.

## 2. Non-goals

- Rapier
- свет
- egui
- оптимизация размера
- кастомные шейдеры

## 3. Предусловия

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install wasm-bindgen-cli   # если Trunk не тянет сам
```

## 4. Спецификация

### 4.1. Структура workspace

```
einstein-lift/
├── Cargo.toml                 # [workspace] members
├── Cargo.lock
├── crates/
│   ├── lift-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── frame.rs
│   │       ├── photon.rs
│   │       └── clock.rs
│   └── lift-app/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── scene.rs
├── web/
│   ├── index.html
│   ├── Trunk.toml
│   └── styles.css
```

> Задания этапов (`00-*.md` … `07-*.md`) лежат в корне репозитория, не в `docs/`.

### 4.2. `lift-core` на этом этапе

Реализовать **полностью** согласно `00-physics-spec.md`:

- `FrameParams { external_g, lift_a }`
- `FrameParams::g_eff() -> f64`
- `FrameParams::displacement(t) -> f64`
- `Mode::{Planet, Rocket, FreeFall, Inertial, Custom}`
- `Mode::params() -> FrameParams`
- `ExperimentClock` с `step()`, `reset()`, авто-паузой
- `PhotonLaunch` и `photon_external()`, `photon_lift()`
- `rindler_null_y()`

Никакого `bevy` в зависимостях `lift-core`. **Это критично.**

### 4.3. Первая сцена

- Пол кабины: `3 × 0.1 × 3` м, центр `(0, -0.05, 0)`
- Две боковые стены по X: тонкие плиты, **полупрозрачные** или каркасные
- Задняя стена по −Z: непрозрачная
- Передняя стена (+Z): отсутствует — это «разрез» для камеры
- Потолок: каркас из рёбер, не сплошная плита
- Три сферы-заглушки радиуса `0.15` на высоте `y = 1.5`
- Направленный свет + ambient
- Камера в позиции `(5, 3.5, 6)`, смотрит в `(0, 1.2, 0)`

Кабина должна читаться как комната, а не как набор кубов.

### 4.4. WASM-диагностика

Обязательно:

```rust
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();
}
```

Без `console_error_panic_hook` любая panic в WASM выглядит как
`unreachable executed` — это делает отладку невозможной.

### 4.5. Версионный контракт

Выполнить процедуру из `overview.md` §6 («Версионный контракт»).
Заполнить раздел «Зафиксированный стек» в `00-physics-spec.md`.

Если `DefaultPlugins` на выбранной версии сам создаёт `<canvas>` —
**не писать ручной JS-инициализатор**. Использовать стандартный путь Trunk.

## 5. Критерии приёмки

- [ ] `cargo test -p lift-core` проходит нативно, ≥ 10 тестов
- [ ] `trunk serve --open` открывает страницу без ошибок консоли
- [ ] В браузере видна кабина с тремя сферами
- [ ] Камера вращается вокруг сцены (OrbitControls / встроенный эквивалент)
- [ ] `cargo build --release --target wasm32-unknown-unknown` succeeds
- [ ] Panic при намеренно вызванной `panic!()` виден в консоли с именем файла
- [ ] Зафиксирована совместимая тройка версий
- [ ] `README.md` содержит рабочую команду запуска

## 6. Тесты `lift-core`

```text
T01.1  g_eff(Planet)  == -9.81
T01.2  g_eff(Rocket)  == -9.81
T01.3  g_eff(FreeFall) == 0.0
T01.4  g_eff(Inertial) == 0.0
T01.5  Planet и Rocket дают побитово одинаковые g_eff
T01.6  displacement(0) == 0
T01.7  displacement(t) для Rocket > 0
T01.8  displacement(t) для FreeFall < 0
T01.9  photon_lift в режиме FreeFall: y(t) == y₀ для 100 значений t
T01.10 photon_lift в режиме Rocket: совпадает с параболой
T01.11 rindler_null_y → парабола при малом аргументе, rel_err < 1e-6
T01.12 rindler_null_y возвращает None за областью определения
T01.13 ExperimentClock останавливается на duration
```

## 7. Типичные ловушки

| Ловушка | Последствие | Как избежать |
|---|---|---|
| `bevy_webgl2` в зависимостях | конфликт плагинов / несуществующий crate | не подключать |
| `default-features = false` без изучения фич | чёрный экран без объяснений | на T01 использовать default |
| Отсутствие `console_error_panic_hook` | нечитаемые panic | добавить сразу |
| `opt-level = "z"` на первом шаге | компиляция по 10 минут | оптимизацию делать на T05 |
| Ручной canvas + автоcanvas от Bevy | два канваса, один пустой | выбрать один путь |
| `f32` в математике ядра | дрейф тестов | в core только `f64` |

## 8. PROMPT

```text
Ты — senior Rust-разработчик, специализирующийся на Bevy и WebAssembly.

ПРОЕКТ
«Лифт Эйнштейна» — обучающее 3D-приложение о принципе эквивалентности.
Архитектура: workspace из двух crate:
  - lift-core: чистый Rust, f64, никакого Bevy/Rapier/wasm-bindgen
  - lift-app:  Bevy + Rapier + egui, компилируется в wasm32-unknown-unknown

ЗАДАЧА T01: поднять фундамент. Ничего кроме него не делать.

ШАГ 1. Версионный контракт
Проверь актуальные совместимые версии bevy, bevy_rapier3d, bevy_egui.
Зафиксируй совместимую тройку. НЕ угадывай номера по памяти.
Если не можешь проверить — явно скажи и оставь TODO(VERIFY).
Не подключай bevy_webgl2: это устаревший плагин.

ШАГ 2. lift-core
Реализуй:
  FrameParams { external_g: f64, lift_a: f64 }
    g_eff() = -external_g - lift_a
    displacement(t) = 0.5 * lift_a * t * t
  Mode { Planet, Rocket, FreeFall, Inertial, Custom }
    Planet:   g_ext=9.81, a=0
    Rocket:   g_ext=0,    a=+9.81
    FreeFall: g_ext=9.81, a=-9.81
    Inertial: g_ext=0,    a=0
  ExperimentClock { t, dt=1/60, running, duration=3.0 }
    step(), reset(), авто-пауза на duration
  PhotonLaunch { origin: [f64;3], c: f64 }
    photon_external(t, launch, g_ext) -> [f64;3]
      x = x0 + c*t
      y = y0 - 0.5*g_ext*t*t
    photon_lift(t, launch, frame) -> [f64;3]
      y = y_external - frame.displacement(t)
  rindler_null_y(x, a, c) -> Option<f64>
      = (c*c/a) * (sqrt(1 - (a*x/(c*c))^2) - 1)
      None если |a*x/(c*c)| >= 1

ШАГ 3. Тесты
Напиши ≥13 unit-тестов, перечисленных в 01-foundation.md §6.
Особенно важны:
  - Planet и Rocket дают одинаковый g_eff
  - в FreeFall фотон в кадре лифта идёт СТРОГО горизонтально

ШАГ 4. lift-app
Создай сцену:
  пол 3×0.1×3, две прозрачные боковые стены, задняя стена,
  каркасный потолок, передняя стена отсутствует (разрез)
  три сферы радиуса 0.15 на высоте 1.5
  DirectionalLight + ambient
  камера (5, 3.5, 6) → target (0, 1.2, 0), орбитальное управление
Добавь console_error_panic_hook и логирование в консоль браузера.

ШАГ 5. Веб-обвязка
web/index.html + web/Trunk.toml + CSS на весь viewport.
Если Bevy сам создаёт canvas — не пиши ручной JS-инициализатор.
Дай точные команды установки инструментов и запуска.

ОГРАНИЧЕНИЯ
- Никакого Rapier, egui, шейдеров и оптимизации размера на этом шаге.
- Не выдумывай API. Сомнительные места помечай TODO(VERIFY).
- Всю математику держи в lift-core.

ФОРМАТ ОТВЕТА
1. Зафиксированные версии и обоснование совместимости
2. Все файлы с полными путями
3. Команды запуска
4. Чек-лист проверки в браузере
5. Список TODO(VERIFY), если есть

ПРАВИЛА ДЛЯ ИИ
Не выдумывай API; не меняй контракт типов из README;
каждая публичная физическая функция — с тестом;
не добавляй зависимости без объяснения;
не пиши «готово» без способа проверки.
```

## 9. Handoff → T02

Передать: зафиксированные версии, пути к `FrameParams`/`Mode`,
рабочую команду `trunk serve`, имя сцены-корня.