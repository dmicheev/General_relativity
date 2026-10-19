# Лифт Эйнштейна

Интерактивное 3D-приложение (браузер, WASM) о **принципе эквивалентности** ОТО.
Обзор проекта, контракты и порядок этапов: [overview.md](overview.md).
Физическая спецификация: [00-physics-spec.md](00-physics-spec.md).

## Структура

- `crates/lift-core` — вся физика (чистый Rust, f64, без зависимостей)
- `crates/lift-app` — Bevy-приложение (сцена, физика, UI), компилируется в WASM
- `web/` — index.html + Trunk.toml + стили
- `00-*.md … 07-*.md` — задания этапов

## Быстрый старт

```bash
# одна-time настройка окружения на этой машине (см. 00-physics-spec.md §6):
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"

# тесты физического ядра (нативно, быстро)
cargo test -p lift-core

# демо траекторий луча в 4 режимах (терминал)
cargo run -p lift-core --example demo

# разработка веб-версии (сервер на http://127.0.0.1:8080)
trunk serve --address 127.0.0.1 --port 8080   # из папки web/

# релизная сборка
trunk build            # из web/, результат в dist/
cargo build --release --target wasm32-unknown-unknown -p lift-app
```

Управление камерой: стрелки — орбита, `+`/`−` — зум, drag мышью — орбита.

## Стек

`bevy 0.20` · `bevy_rapier3d 0.37` · `bevy_egui 0.43` · `trunk 0.21.14`
(совместимость проверена по crates.io, см. 00-physics-spec.md §6)
