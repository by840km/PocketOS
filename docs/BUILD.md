# Сборка и следующий технический шаг

## Rust

На Windows с Rust и рабочим линкером (MSVC либо GNU). Для GNU в .cargo/config.toml выбран встроенный rust-lld: это обходит проблему MinGW с кириллицей в пути.

```powershell
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
.\packaging\build-portable.ps1
```

Скрипт не скачивает runtime и не форматирует накопители.

## Runtime и гостевой образ

1. Подготовить Linux build-host (VM или отдельный Linux). Linux image нельзя считать собранным только по наличию Rust EXE.
2. Установить Debian Stable x64 в отдельный BIOS-совместимый QCOW2-диск с virtio storage/network, KDE Plasma и готовой учётной записью. Начальная установка выполняется разработчиком в рабочей VM, конечному пользователю выдаётся установленный образ.
3. Оставить образ компактным: флешка проекта около 29.3 GiB, начальная цель виртуального диска 20 GiB. Учитывать внешние файлы, рост диска и резерв свободного места. Автоматизированный рецепт установки — следующий deliverable.
4. Проверить обычный KDE Plasma, сеть, размер экрана, ввод, выключение. Снять показатели TCG и WHPX. Не включать 3D-ускорение до проверки поддержки Windows-сборкой QEMU.
5. Выбрать и зафиксировать QEMU Windows x64 с GUI, DLL, SeaBIOS и virtio VGA. Полную папку runtime разместить в qemu/; текущий doctor ожидает share/bios-256k.bin. Контрольные суммы/лицензии и фактическая версия обязательны перед распространением.
6. Поместить готовый диск в system/pocketos.qcow2. В переносном комплекте выполнять `pocketos-core.exe doctor`, затем `PocketOS.exe`.
7. WHPX использовать только если он уже доступен: `PocketOS.exe start --accel whpx`. По умолчанию QEMU пробует WHPX, затем TCG. Свободная RAM пока автоматически не проверяется.

Исходники и документация:
- https://www.qemu.org/docs/master/system/invocation.html
- https://invent.kde.org/plasma/plasma-desktop
- https://invent.kde.org/plasma/kwin
- https://rclone.org/commands/rclone_mount/

## Облако

Шаблон system/cloud/mount-cloud.sh запускается только внутри Linux после ручной настройки remote `pocketcloud`. Он не содержит учётных данных и не создаёт сервер Nextcloud. Пока не запускать с ценными документами: управление завершением, конфликты и защита хранилища ещё не реализованы.



Профиль и фактический путь сборки Debian/KDE Plasma: [system/image/README.md](../system/image/README.md). Исходный KDE Plasma проверен в WHPX; полноценный бенчмарк TCG и свой Shell ещё впереди.

## KDE 0.4

Профиль NoCloud теперь включает закреплённые файлы из system/plasma через make-seed.py. configure-guest.sh устанавливает KDE, затем запускает plasma/configure.sh. Для обновления уже собранной системы использовать отдельный writable QCOW2 overlay, установить KDE, удалить GNOME, применить конфиг и проверить SDDM/Plasma. Никогда не изменять readonly USB-базу во время её работы.

Перед финальной конвертацией: завершить GUI-сеанс, удалить сборочные ключи/данные, отключить SSH, очистить apt и выполнить fstrim в build VM с discard=unmap. Выключить VM. Через qemu-img convert создать самостоятельный QCOW2 без backing file. Для readonly USB можно использовать сжатие zstd; окончательный выбор проверять загрузкой с реального USB.

Собрать минимальный initrd по system/ramroot/README.md, проверить debug-сборкой реальный Plasma/renderer, затем отдельной загрузкой release без ключа. Применение темы не запускает внешние curl/install-скрипты: в образ копируются закреплённая палитра и собственные конфиги. Режим build=rw не доступен в release initrd без debug-ключа.
