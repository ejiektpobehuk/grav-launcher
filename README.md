## ГРАВ launcher

Консольная программа для обновления и запуска игры [ГРАВ](https://arigven.games/grav/).

Игру разрабатывает [@ARIGVEN](https://github.com/arigven), не я.

Игра ещё на ранних этапах разработки и не распространяется через какие-либо игровые площадки.
Поэтому обновления приходится загружать вручную, что на SteamDeck'e делать неудобно.

`grav-launcher` автоматизирует процесс обновления игры.

ЗДЕСЬ БУДЕТ ПРЕВЬЮ РАБОТЫ

### Установка на Steam Deck

1. Переключиться в Desktop Mode
1. Загрузить файл `grav-launcher` из [релизов](https://github.com/ejiektpobehuk/grav-launcher/releases)
1. Сделать его исполняемым
    1. В проводнике открыть его контекстное меню - `L2` при курсоре на файле
    1. Выбрать в контекстном меню Свойства / `Properties`
    1. В окне свойств переключиться на вкладку `Permissions`
    1. Поставить галочку `Is executable`
    1. Принять изменения - `Ok`
1. Добавить в Steam - в контекстном меню файла `Add to Steam`

### Терминал

`grav-launcher` - консольное приложение.
Ему для работы нужен эмулятор терминала.
Для упрощения установки на SteamDeck, launcher умеет определять, запущен ли он в терминале:

- При запуске напрямую (например, из Steam) он автоматически откроет терминал
- При запуске из терминала он запустится в существующем окне терминала
- Если по каким-то причинам требуется отключить автоматический запуск в терминале, можно использовать параметр `--no-terminal`

### Roadmap

- [x] сборка бинаря в релизах
- [ ] pin версии Rust и автоматизация для обновления
- [x] поддержка контроллера для навигации по логам и выхода
- [ ] навигация по логам
- [x] обновление самого launcher'а из launcher'a
- [ ] graceful shutdown
