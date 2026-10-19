# Baseline производительности (T05)

## Версия стека

| Компонент | Версия |
|---|---|
| Дата замера | 2026-10-18 |
| bevy | 0.20.0 |
| bevy_rapier3d / rapier3d | 0.37.0 / 0.36.1 |
| wasm-bindgen | 0.2 |
| trunk | 0.21.14 (wasm-bindgen + binaryen `wasm-opt version_123` из кеша Trunk) |
| rustc | stable-aarch64-apple-darwin (rustup), wasm32-unknown-unknown |
| Профиль | `[profile.release]`: lto="fat", codegen-units=1, strip=true, opt-level="z" |

## Таблица «до / после»

| Метрика | До (baseline) | После (T05) | После T06 (+egui) |
|---|---:|---:|---:|
| WASM raw | 60.5 МБ | 14.3 МБ | **20.06 МБ** |
| WASM gzip -9 | 15.2 МБ | 5.4 МБ | **7.30 МБ** (бюджет ≤ 8 ✓) |
| JS glue raw / gzip | — | 103 / 14 КБ | 110 / 14 КБ |

T06 вернул UI: `bevy_egui 0.43.1` (фичи render + default_fonts,
+1.9 МБ gzip). Диета фич — в разделе «Изменения против буквы T06»
в `00-physics-spec.md`.

### Состав оптимизации «после»

1. **Диета зависимостей** (T05 §4.1):
   - `bevy_egui` удалён (не был подключён в приложении; вернётся на T06);
   - `bevy`: `default-features = false`, минимум: std, multi_threaded,
     async_executor, bevy_winit, bevy_asset, bevy_log, bevy_render,
     bevy_core_pipeline, bevy_pbr, bevy_gizmos(_render), webgl2,
     tonemapping_luts + zstd_rust. Выключены: audio, gltf, animation,
     ui/text, gilrs, state, scene, picking, default_font, ktx2-декодеры
     (кроме LUT), custom_cursor, clipboard, sysinfo;
   - `bevy_rapier3d`: только `dim3` (async-collider, debug-render-3d,
     picking-backend, to-bevy-mesh не используются).
2. **Профиль релиза** (T05 §4.2): lto="fat", codegen-units=1, strip=true.
   `panic="abort"` НЕ включён — проверяемая диагностика паник
   (console_error_panic_hook) важнее ~1 МБ.
3. **wasm-opt** (T05 §4.3): бинарник из кеша Trunk, вызывается Trunk'ом
   (`data-wasm-opt="z" data-wasm-opt-params="--all-features"` в
   `web/index.html`) — единственное место применения. `--all-features`
   обязателен: свежий binaryen требует флагов bulk-memory/nontrapping на
   выходе rustc/wasm-bindgen.

### opt-level: 3 vs z

| Вариант | raw | gzip | frame time |
|---|---:|---:|---|
| opt-level = 3 | 24.2 МБ | 8.3 МБ | не измерен (headless) |
| opt-level = "z" | 14.3 МБ | 5.4 МБ | не измерен (headless) |

**Выбран z**. Обоснование не «потому что меньше»: hot path приложения —
вызовы gizmos и аналитика фотона/трейлов (O(сотни) точек), физика —
fixed-step 1/60 на 3 телах; ни один из них не числодробительный.
Числовых ядер, чувствительных к opt-level, в приложении нет.
**Условие пересмотра**: браузерный прогон (T06/T07) покажет < 50 FPS
на 1080p → вернуть 3 (gzip станет 8.3 МБ — бюджет закрывается
brotli-компрессией CDN, типично ~6.3 МБ).

### Рендер-гигиена (T05 §5)

- три груза делят один `Sphere` Mesh-handle (было 3 копии);
- трейлы/фотон — gizmo-линестрипы (unlit, один батч на линию), не PBR;
- скрайт-буферы `Local<Vec<_>>` с clear/extend — ноль аллокаций `Vec`
  в `draw_trails` / `draw_photon` (было: новый Vec на каждую линию
  каждый кадр);
- shadow maps directional light выключены по умолчанию (bevy_light
  default `shadow_maps_enabled: false`) — не включали;
- `DirectionalLight` один, `AmbientLight` один.

## Как измерять (воспроизведение)

```bash
tools/bench.sh                # сборка trunk --release + замер raw/gzip
tools/bench.sh --no-build     # замер существующего dist/
```

## Frame time p50/p95 и FPS (шаблон — заполняется браузерным прогоном)

| Метрика | Значение | Как |
|---|---|---|
| Time to first frame | _заполнить_ | Performance API / ручной timestamp в консоли |
| FPS p50 / p95 (1080p) | _заполнить_ | `bevy_dev_tools` FPS-оверлей или ручной замер |
| Draw calls | _заполнить_ | рендер-статистика бэкенда |
| Heap (Chromium, эвристика) | _заполнить_ | `performance.memory` |

> Замеры производительности в браузере — этап T06/T07 (нужен деплой);
> headless-замер FPS не репрезентативен (software-rasterizer).

## Детерминизм после оптимизации

`cargo test -p lift-core` (47) + `cargo test -p lift-app` (28 unit +
15 интеграционных) — все проходят с UI-слоем, включая T02.7
(10 прогонов — одинаковый хеш) и T04.11/T04.12. Физика не изменилась:
UI не пишет в симуляцию кроме легальных каналов (UiEvents/PhotonEvents →
sim_pre).

## Что НЕ оптимизировали и почему

- **Многопоточность / rayon / SharedArrayBuffer** — запрещено спекой
  для MVP (ломает static hosting, требует COOP/COEP).
- **Zero-copy JS↔WASM** — не критический путь: физика и рендер в одном
  модуле, через JS-границу по кадрам ничего не передаётся (§2).
- **Instancing / LOD / ручной batching трейлов** — сцена: 3 груза,
  3 пружины, gizmo-линии; draw calls единицы. Усложнение без причины.
- **Частичный GPU-buffer update** — gizmos immediate-mode, своих
  вершинных буферов на кадр у нас нет (это уже «батч без аллокаций»).
- **panic="abort"** — теряется диагностика паник (см. выше).
