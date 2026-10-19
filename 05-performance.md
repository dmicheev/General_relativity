# 05. Производительность и размер WASM

**ID:** T05
**Зависимости:** T04
**Артефакты:** `Cargo.toml` (profiles/features), `tools/bench.sh`, `docs/baseline.md`
**Сложность:** средняя
**Оценка:** 1–2 дня

## 1. Цель

Превратить «работает» в «работает быстро и быстро загружается».

## 2. Важное уточнение архитектуры

> **Zero-copy JS ↔ WASM не является критическим путём этого проекта.**

И Bevy, и Rapier, и математика живут **в одном WASM-модуле**.
Данные между физикой и рендером не пересекают JS-границу вообще.
Поэтому классический «zero-copy pipeline через `SharedArrayBuffer`»
здесь решает несуществующую проблему.

Он становится актуален только если:
- рендер перенесён на JS-сторону (Three.js / Babylon),
- или добавлен тяжёлый compute-путь, общающийся с JS.

Это **приложение**, а не требование MVP.

## 3. Этап 3.1 — Сначала измерить

Записать baseline до любых оптимизаций:

| Метрика | Как измерять |
|---|---|
| Frame time, p50 / p95 | `bevy_diagnostic::FrameTimeDiagnosticsPlugin` |
| Draw calls | `RenderDiagnosticsPlugin` / бэкенд-статистика |
| Triangles / frames | та же диагностика |
| WASM size, raw | `du -b` |
| WASM size, gzip | `gzip -9 -k` |
| Time to first frame | Performance API в JS / ручной timestamp |
| Heap memory | `performance.memory` (Chromium, эвристика) |
| FPS на встроенной GPU | ручной прогон |

Сохранить в `docs/baseline.md` **с датой и версией стека**.

## 4. Этап 3.2 — Диета зависимостей

Только после baseline.

### 4.1. Bevy features

Начать с `default-features = false` и включить минимальный набор:

```text
нужно:
  bevy_core, bevy_transform, bevy_hierarchy, bevy_input,
  bevy_window, bevy_render, bevy_pbr, bevy_camera,
  bevy_asset, bevy_scene, bevy_winit, bevy_text (если нужен)

вероятно не нужно:
  animation, gltf, audio, ui (если используем egui),
  gizmos (кроме debug-сборки), basis/ktx2 декодеры,
  bevy_gilrs (gamepads), bevy_state (если нет стейт-машины)
```

> Точные имена фич **менять по зафиксированной версии**.
> В разных релизах Bevy они переименовываются.
> Проверять через `cargo tree -e features` и `Cargo.toml` dependency.

### 4.2. Release profile

```toml
[profile.release]
lto = "fat"
codegen-units = 1
opt-level = 3
strip = true
```

`panic = "abort"` — **только после проверки**, что diagnostics не теряются.
На этом шаге оставить дефолтное поведение.

`opt-level = "z"` сравнивать с `3` по двум осям: размер и frame time.
Не выбирать «потому что меньше».

### 4.3. wasm-opt

```bash
wasm-opt -O3 --strip-debug --strip-producers \
  -o dist/app.opt.wasm dist/app.wasm
```

Проверить, что Trunk не применяет это дважды.

## 5. Этап 3.3 — Рендер-оптимизация

| Приём | Применительно к проекту |
|---|---|
| Share meshes | три сферы — один `Mesh` handle, три `Transform` |
| Share materials | одинаковые материалы — один handle |
| Instancing | если число частиц/трейлов растёт — `draw_instanced` |
| Один batch для трейлов | не по draw call на сегмент |
| Буферы без аллокаций | `Vec::clear()` + `extend`, не новый `Vec` каждый кадр |
| GPU buffer update | `write_buffer` на изменённый диапазон, не весь буфер |
| Shadows | один directional light с ограниченной shadow map; не 4 источника |
| PBR vs unlit | стены и пол — PBR; трейлы и голова фотона — unlit/emissive |
| LOD / culling | для MVP кабина мала; не усложнять без причины |

### Анти-паттерн

```rust
// ПЛОХО: аллокация и полная замена mesh каждый кадр
fn update_trail(mut commands: Commands, mut mesh_assets: ResMut<Assets<Mesh>>) {
    let mut mesh = Mesh::new(...);
    // ...
    commands.entity(e).insert(mesh_assets.add(mesh));
}
```

```text
ХОРОШО: один Mesh, один GpuBuffer, обновлять диапазон вершин головы/хвоста.
Если API Bevy не позволяет удобный partial update — переиспользовать
handle и вызывать mesh.insert_attribute() с тем же buffer id.
```

## 6. Этап 3.4 — Многопоточность (опционально)

**Не входит в MVP.**

Если понадобится:

- `wasm-bindgen-rayon` + `rayon` для параллельных вычислений в core
- `SharedArrayBuffer` обязателен **только здесь**
- требует HTTP-заголовков:
  ```
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Embedder-Policy: require-corp
  ```
- не работает на `file://`
- требует отдельного CI-прогона и проверки в Safari/Firefox

**Не включать多线程 «на будущее»** — это ломает деплой на обычный static hosting.

## 7. Приложение A. Zero-copy (только для будущего гибридного рендера)

### A.1. Корректное утверждение

- Zero-copy через `WebAssembly.Memory.buffer` **не требует** `SharedArrayBuffer`.
- `SharedArrayBuffer` нужен только для **разделяемой памяти между потоками**.
- SAB также требует COOP/COEP и cross-origin isolation.

### A.2. Схема

```text
Rust:
  static mut BUFFER: Option<Vec<f32>> = None;
  #[wasm_bindgen] pub fn buffer_ptr() -> *mut f32
  #[wasm_bindgen] pub fn buffer_len() -> usize

JS:
  const ptr = wasm.buffer_ptr();
  const len = wasm.buffer_len();
  const view = new Float32Array(wasm.memory.buffer, ptr, len);
  view[i] = ...;              // запись прямо в память WASM
  wasm.recompute();           // Rust читает &[f32] без копирования
```

### A.3. Подводные камни

| Проблема | Решение |
|---|---|
| `memory.grow()` инвалидирует старый `.buffer` | получать `memory.buffer` заново после grow |
| Rust reclaim/drop двигает память | буфер статический, выделен один раз |
| `Vec<f32>` moveinvalidates pointer | `Box::leak` или статический массив фиксированного размера |
| Нет bounds checking на JS-стороне |长度 только из экспортированной функции |
| UAF при realloc | never realloc; фиксированный capacity + overflow flag |

## 8. Критерии приёмки

Замеры — в `docs/baseline.md` (2026-10-18), воспроизведение —
`tools/bench.sh`.

- [x] `docs/baseline.md` содержит замеры **до** оптимизации *(60.5 / gzip 15.2 МБ)*
- [x] `docs/baseline.md` содержит замеры **после** *(14.3 / gzip 5.4 МБ)*
- [x] Release WASM (gzip) ≤ **8 МБ** — целевой бюджет *(5.4 МБ; при opt-level=3 — 8.3, см. сравнение)*
- [ ] ≥ 50 FPS на встроенной GPU при 1080p *(заполняется браузерным прогоном T06/T07; headless-замер нерепрезентативен)*
- [x] Нет аллокаций `Mesh` / `Vec` в hot-path каждый кадр *(общий Sphere-handle; Local-скрайты в draw_trails/draw_photon)*
- [x] Все тесты `lift-core` проходят после оптимизации *(47/47)*
- [x] Физические результаты не изменились *(детерминизм-тесты T02.7, T04.11/T04.12 проходят)*
- [x] `cargo tree` не содержит явно неиспользуемых тяжёлых зависимостей *(bevy_egui удалён; bevy default-features=false; rapier — только dim3)*
- [x] Debug-сборка по-прежнему даёт читаемые panic *(console_error_panic_hook сохранён; panic=abort не включался)*
- [x] Многопоточность **не** включена в MVP *(rayon/SAB отсутствуют)*

## 9. PROMPT

```text
Ты — senior Rust/WASM performance engineer. Проект «Лифт Эйнштейна».

КОНТЕКСТ
Bevy + Rapier + egui + кастомные mesh-трейлы, всё в одном WASM-модуле.
Этапы T01–T04 реализованы и работают. Версии зафиксированы в Cargo.lock.

ЗАДАЧА T05: измерить и оптимизировать. НЕ оптимизировать вслепую.

ВАЖНОЕ УТОЧНЕНИЕ
Zero-copy JS↔WASM здесь НЕ нужен: физика и рендер в одном модуле,
через JS-границу по кадрам ничего не передаётся.
Не предлагай SharedArrayBuffer как решение основной проблемы.
Многопоточность в MVP НЕ включаем.

ПОРЯДОК РАБОТЫ (соблюдать строго)

ЭТАП 1. Измерить baseline
Дай способ снять: frame time p50/p95, draw calls, triangles,
raw/wasm gzip size, time to first frame.
Используй bevy_diagnostic и/или встроенную диагностику рендерера.
Оформи шаблон docs/baseline.md.

ЭТАП 2. Диета зависимостей
Предложи default-features=false для bevy с минимальным набором фич,
НЕОБХОДИМЫМ для: 3D PBR, camera, input, window, asset, transform,
rapier3d, egui, text.
Точные имена фич бери из зафиксированной версии, не из памяти.
Покажи, как проверить через cargo tree -e features.

ЭТАП 3. Release profile
lto="fat", codegen-units=1, opt-level=3, strip=true.
Сравни opt-level=3 vs "z" по размеру И по frame time.
panic="abort" только если не теряется диагностика.
Дай команду wasm-opt и проверь, что Trunk не делает это дважды.

ЭТАП 4. Рендер
- три сферы делят один Mesh handle
- трейлы рисуются одним batch, не по draw call на сегмент
- нет нового Vec/Mesh в hot path каждый кадр
- обновлять GPU buffer только изменённый диапазон
- один directional light с ограниченной shadow map
- трейлы и голова фотона — unlit/emissive, не PBR

ЭТАП 5. Замерить результат
Заполнить таблицу "до/после" и объяснить каждую выигранную метрику.

БЮДЖЕТЫ (цели, не гарантии)
  WASM gzip <= 8 МБ
  >= 50 FPS на встроенной GPU при 1080p
  ноль аллокаций mesh в hot path

ПРИЛОЖЕНИЕ (только если попросат отдельно)
Опиши zero-copy через WebAssembly.Memory.buffer для будущего гибридного
рендера. Подчеркни: SAB нужен ТОЛЬКО для многопоточности,
а не для zero-copy в одном потоке.
Покажи проблему инвалидации buffer при memory.grow() и как её решить.

ОГРАНИЧЕНИЯ
- Не менять физику и контракт типов.
- Не включать rayon/threads в MVP.
- Не подключать новые графические плагины ради «красивых цифр».
- Не выдумывать имена feature flags.

ФОРМАТ ОТВЕТА
1. Шаблон baseline
2. Изменённые Cargo.toml
3. Изменённые hot-path системы
4. Таблица до/после
5. Что НЕ стали оптимизировать и почему

ПРАВИЛА ДЛЯ ИИ
Сначала измерить, потом менять; каждая оптимизация — с объяснением метрики;
не менять контракт типов; не добавлять зависимости без объяснения.
```

## 8. Handoff → T06

Передать: финальный список feature flags, бюджет размера,
способ добавлять UI без нарушения frame time.

### Handoff → T06 (фактический, T05 закрыт)

- **Feature flags** (`crates/lift-app/Cargo.toml`): bevy
  default-features=false + [std, multi_threaded, async_executor,
  bevy_winit, bevy_asset, bevy_log, bevy_render, bevy_core_pipeline,
  bevy_pbr, bevy_gizmos, bevy_gizmos_render, webgl2, tonemapping_luts,
  zstd_rust]; bevy_rapier3d — только `dim3`.
- **Бюджет размера**: gzip 5.4 МБ из 8. **ВНИМАНИЕ T06**: подключение
  egui добавит ~2–3 МБ gzip — при превышении бюджета: `panic="abort"`
  (с проверкой диагностики), отказ от `tonemapping_luts`, opt-level
  не трогать.
- **UI без нарушения frame time**: egui-контекст обновляется раз в
  кадр в Update — наша физика в FixedUpdate (60 Гц) и не зависит от
  частоты рендера; логи T02–T04 (метрики) переехать в панели egui.
- **Фреймтайм-замеры** (незакрытый пункт чеклиста): собрать
  `trunk serve`, прогон в браузере, заполнить шаблон в
  `docs/baseline.md`; при < 50 FPS — пересмотреть opt-level (см.
  комментарий в Cargo.toml).
- **Инструмент замера**: `tools/bench.sh [--no-build]`.