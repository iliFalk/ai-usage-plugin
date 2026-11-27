// Omarchy UI message catalog. Chrome and formatter copy live here; report
// metric labels from Rust stay as they arrive. Locale "auto" follows
// Qt.locale() / $LANG.

var DEFAULT_LOCALE = "en"
var SUPPORTED = ["en", "ru", "pt-BR", "ko", "es"]

var MONTHS = {
  en: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
  ru: ["янв", "фев", "мар", "апр", "мая", "июн", "июл", "авг", "сен", "окт", "ноя", "дек"],
  "pt-BR": ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"],
  ko: ["1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월", "12월"],
  es: ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"]
}

// English wire credential notes remapped for display only.
var NOTE_KEYS = {
  "usage quota": "credentials.note.usage_quota"
}

var MESSAGES = {
  en: {
    "app.name": "AI usage",
    "hero.settings": "Settings",
    "hero.settings_meta": "Display, provider & API keys",
    "hero.settings_detail": "Existing configuration stays in place until you save.",
    "hero.usage_limits": "Usage and limits",
    "hero.loading": "Loading providers",
    "hero.usage_report": "Usage report",
    "hero.provider_unavailable": "Provider unavailable",
    "action.refresh": "Refresh usage",
    "action.settings": "Settings",
    "action.back": "Back to usage",
    "action.retry": "Retry",
    "action.save": "Save settings",
    "action.saving": "Saving…",
    "section.usage": "USAGE",
    "section.usage_balance": "USAGE & BALANCE",
    "section.settings": "SETTINGS",
    "section.display": "DISPLAY",
    "section.bar_window": "TOP BAR WINDOW",
    "section.language": "LANGUAGE",
    "section.primary": "PRIMARY PROVIDER",
    "section.providers": "PROVIDERS",
    "section.credentials": "CREDENTIALS",
    "loading.config": "Loading configuration…",
    "loading.providers": "Collecting configured providers…",
    "empty.no_usage": "No configured provider reported usage.",
    "status.cached": "Cached data · the provider could not supply a fresh response.",
    "status.refresh_failed": "Refresh failed; showing the previous report. {error}",
    "status.filter_miss": "No configured entry matches ‘{id}’. Clear the provider setting or use an id from ai-usagebar usage --json.",
    "status.saved": "Settings saved. Usage is refreshing.",
    "status.vendor_on": "On — included in the report.",
    "status.vendor_off": "Off — not fetched.",
    "status.will_clear": "will clear",
    "status.env_set": "set in the environment",
    "status.inline_set": "stored inline",
    "status.not_set": "not set",
    "toggle.show_value": "Show usage value in the top bar",
    "toggle.show_value_desc": "Turn this off for an icon-only bar entry. The panel and tooltip still show full usage details. Applies immediately.",
    "toggle.show_provider": "Show provider name in the top bar",
    "toggle.show_provider_desc": "Turn this on to prefix the bar entry with the provider's short code — ocg, opr, dsk. Off by default. Applies immediately.",
    "toggle.show_all": "Show all providers in the top bar",
    "toggle.show_all_desc": "Turn this on to show every configured provider's icon and usage in the top bar at once, instead of cycling one at a time. Click still opens the panel; the wheel still selects which details you see. Off by default. Applies immediately.",
    "toggle.color_code": "Color-code usage by level",
    "toggle.color_code_desc": "Paint bar values, panel meters, and the tooltip green → yellow → orange → red as usage climbs, using your Omarchy theme. Turn off for a single foreground color everywhere. Off by default. Applies immediately.",
    "bar_window.help": "Which quota the bar shows. Providers lacking it fall back to highest. Applies immediately.",
    "bar_window.auto": "Highest (auto)",
    "bar_window.session": "5-hour (session)",
    "bar_window.weekly": "7-day (weekly)",
    "bar_window.monthly": "Monthly (monthly)",
    "toggle.brand_icons": "Show provider logos",
    "toggle.brand_icons_desc": "Draws each provider's own mark in the top bar and panel. Turn this off for the generic icon the bar used before: one robot for a single provider, and the provider's short code for each chip when \"Show all providers in the top bar\" is on. On by default. Applies immediately.",
    // Quota-threshold notifications ([notifications] in config.toml).
    "section.notifications": "NOTIFICATIONS",
    "notify.enabled": "Notify when a quota window nears its limit",
    "notify.enabled_desc": "Sends one desktop notification when a window crosses the threshold below, and warns 48 hours before banked credits expire. One notification per crossing; it re-arms only after usage drops 7 points.",
    "notify.threshold": "Threshold (%)",
    "notify.threshold_desc": "A whole number from 1 to 100. At 100 only an already exhausted window notifies.",
    "notify.threshold_invalid": "Enter a whole number from 1 to 100.",
    "status.notify_off": "off",
    "error.notify_threshold": "The notification threshold must be a whole number between 1 and 100.",
    "section.metrics": "METRICS",
    "metrics.help": "Choose which metrics each provider shows. A metric you switch off disappears from the panel, the top bar and the tooltip, and is ignored when the bar picks the highest percent. The last metric still on cannot be switched off. Applies immediately.",
    "section.show_as": "SHOW USAGE AS",
    "show_as.help": "Whether percentages read what you have used or what is left of the same window. Applies immediately.",
    "show_as.used": "Used",
    "show_as.left": "Left (remaining)",
    "metric.left": "{percent}% left",
    "monthly_cycle_day": "Monthly cycle reset day",
    "monthly_cycle_day_desc": "Day of the month (1-28) the OpenCode Go billing cycle resets on. Leave unset to estimate a 30-day cycle.",
    "monthly_cycle_day_unset": "Not set",
    "language.help": "Language for the panel and settings. Auto matches your system language.",
    "language.auto": "System (auto)",
    "language.en": "English",
    "language.ru": "Русский",
    "language.pt-BR": "Português (Brasil)",
    "language.ko": "한국어",
    "language.es": "Español",
    "primary.help": "Used by the CLI and as this panel's preferred provider.",
    "providers.help": "Which providers are fetched at all. Turn one off and it leaves the bar, panel and reports until you switch it back on; turning one on takes effect on the next refresh. Saving a credential for a provider keeps switching it on.",
    "providers.bar_title": "Providers on the bar",
    "providers.bar_help": "Choose which reported entries appear on the bar, including named accounts. Changes save immediately. Hidden entries stay in the panel. All built-in providers, including inactive ones, remain under Advanced: provider fetching.",
    "providers.shown": "Shown on the bar.",
    "providers.hidden": "Hidden from the bar.",
    "providers.fetch_title": "Advanced: provider fetching",
    "providers.none_shown": "AI Usage · no providers shown on the bar",
    "credentials.help": "Stored values are never loaded into the shell. Leave a field blank to keep its current value, or use the clear button to remove an inline credential. Environment variables take precedence.",
    "credentials.keep_blank": "Leave blank to keep current credential",
    "credentials.paste": "Paste {label}",
    "credentials.keep_key": "Keep the stored key",
    "credentials.clear_key": "Clear the stored inline key",
    "credentials.credential": "credential",
    "credentials.new_key": "new key",
    "credentials.env_override": "environment override",
    "credentials.stored": "stored",
    "credentials.not_configured": "not configured",
    "detail.used_of": "{used} of {limit} used ({percent}%)",
    "credentials.api_key": "API key",
    "credentials.note.usage_quota": "usage quota",
    "error.binary_old": "This installed ai-usagebar binary predates native settings. Update the package.",
    "error.apply": "The settings command did not confirm the save.",
    "graphic.title": "Expected vs actual usage",
    "graphic.expected": "Expected (linear)",
    "graphic.weekly": "Weekly",
    "graphic.monthly": "Monthly",
    "tip.cached": "cached",
    "ready": "Ready",
    "error": "Error",
    "reset.due": "Reset due",
    "reset.date": "{month} {day}",
    "reset.in": "Resets in {duration} · {clock}",
    "updated.unavailable": "Updated time unavailable",
    "updated.just_now": "Updated just now",
    "updated.ago": "Updated {duration} ago",
    "duration.now": "now",
    "duration.days_hours": "{days}d {hours}h",
    "duration.hours_minutes": "{hours}h {minutes}m",
    "duration.minutes": "{minutes}m"
  },
  ru: {
    "app.name": "AI usage",
    "hero.settings": "Настройки",
    "hero.settings_meta": "Отображение, провайдер и ключи",
    "hero.settings_detail": "Пока вы не нажмёте «Сохранить», ничего не изменится.",
    "hero.usage_limits": "Использование и лимиты",
    "hero.loading": "Загрузка провайдеров",
    "hero.usage_report": "Отчёт об использовании",
    "hero.provider_unavailable": "Провайдер недоступен",
    "action.refresh": "Обновить данные",
    "action.settings": "Настройки",
    "action.back": "Назад к использованию",
    "action.retry": "Повторить",
    "action.save": "Сохранить",
    "action.saving": "Сохранение…",
    "section.usage": "ИСПОЛЬЗОВАНИЕ",
    "section.usage_balance": "ИСПОЛЬЗОВАНИЕ И БАЛАНС",
    "section.settings": "НАСТРОЙКИ",
    "section.display": "ОТОБРАЖЕНИЕ",
    "section.bar_window": "ОКНО НА ПАНЕЛИ",
    "section.language": "ЯЗЫК",
    "section.primary": "ОСНОВНОЙ ПРОВАЙДЕР",
    "section.providers": "ПРОВАЙДЕРЫ",
    "section.credentials": "УЧЁТНЫЕ ДАННЫЕ",
    "loading.config": "Загрузка конфигурации…",
    "loading.providers": "Опрос настроенных провайдеров…",
    "empty.no_usage": "Ни один настроенный провайдер не вернул данные.",
    "status.cached": "Кэш · провайдер не смог отдать свежий ответ.",
    "status.refresh_failed": "Обновление не удалось; показан прошлый отчёт. {error}",
    "status.filter_miss": "Нет записи «{id}». Сбросьте провайдер в настройках или укажите id из ai-usagebar usage --json.",
    "status.saved": "Настройки сохранены. Данные обновляются.",
    "status.vendor_on": "Вкл. — участвует в отчёте.",
    "status.vendor_off": "Выкл. — не опрашивается.",
    "status.will_clear": "будет удалено",
    "status.env_set": "задано в окружении",
    "status.inline_set": "сохранено в конфиге",
    "status.not_set": "не задано",
    "toggle.show_value": "Показывать значение на панели",
    "toggle.show_value_desc": "Выключите, чтобы оставить только иконку. Панель и подсказка по-прежнему показывают полные данные. Применяется сразу.",
    "toggle.show_provider": "Показывать имя провайдера на панели",
    "toggle.show_provider_desc": "Добавляет короткий код провайдера — ocg, opr, dsk. По умолчанию выкл. Применяется сразу.",
    "toggle.show_all": "Показывать всех провайдеров на панели",
    "toggle.show_all_desc": "Показывает иконки и usage всех настроенных провайдеров сразу, без переключения по одному. Клик открывает панель; колёсико выбирает детали. По умолчанию выкл. Применяется сразу.",
    "toggle.color_code": "Раскрашивать usage по уровню",
    "toggle.color_code_desc": "Красит значения на панели, полоски в панели и подсказку зелёный → жёлтый → оранжевый → красный по заполнению квоты (цвета темы Omarchy). Выключите для одного цвета интерфейса. По умолчанию выкл. Применяется сразу.",
    "bar_window.help": "Какую квоту показывает панель. Если окна нет — берётся наибольшая. Применяется сразу.",
    "bar_window.auto": "Наибольшая (auto)",
    "bar_window.session": "5 часов (session)",
    "bar_window.weekly": "7 дней (weekly)",
    "bar_window.monthly": "Месяц (monthly)",
    "toggle.brand_icons": "Показывать логотипы провайдеров",
    "toggle.brand_icons_desc": "Рисует фирменный знак каждого провайдера на панели и в окне. Выключите, чтобы вернуть прежнюю общую иконку: один робот для одного провайдера и короткий код провайдера на каждом значке при включённом «Показывать всех провайдеров на панели». По умолчанию вкл. Применяется сразу.",
    // Уведомления о пороге квоты ([notifications] в config.toml).
    "section.notifications": "УВЕДОМЛЕНИЯ",
    "notify.enabled": "Уведомлять о приближении к лимиту",
    "notify.enabled_desc": "Присылает одно уведомление, когда окно квоты пересекает порог ниже, и предупреждает за 48 часов до сгорания накопленных кредитов. Одно уведомление на пересечение; повторно — только после падения на 7 пунктов.",
    "notify.threshold": "Порог (%)",
    "notify.threshold_desc": "Целое число от 1 до 100. При 100 уведомляет только уже исчерпанное окно.",
    "notify.threshold_invalid": "Введите целое число от 1 до 100.",
    "status.notify_off": "выкл.",
    "error.notify_threshold": "Порог уведомлений должен быть целым числом от 1 до 100.",
    "section.metrics": "МЕТРИКИ",
    "metrics.help": "Выберите, какие метрики показывает каждый провайдер. Выключенная метрика пропадает с панели, из окна и подсказки и не учитывается при выборе наибольшего процента. Последнюю включённую метрику выключить нельзя. Применяется сразу.",
    "section.show_as": "КАК ПОКАЗЫВАТЬ",
    "show_as.help": "Проценты показывают, сколько использовано или сколько осталось в том же окне. Применяется сразу.",
    "show_as.used": "Использовано",
    "show_as.left": "Осталось",
    "metric.left": "осталось {percent}%",
    "monthly_cycle_day": "День сброса месячного цикла",
    "monthly_cycle_day_desc": "День месяца (1–28), в который сбрасывается цикл выставления счетов OpenCode Go. Если не задано — цикл оценивается как 30 дней.",
    "monthly_cycle_day_unset": "Не задано",
    "language.help": "Язык панели и настроек. «Авто» совпадает с языком системы.",
    "language.auto": "Системный (авто)",
    "language.en": "English",
    "language.ru": "Русский",
    "language.pt-BR": "Português (Brasil)",
    "language.ko": "한국어",
    "language.es": "Español",
    "primary.help": "Используется CLI и как предпочтительный провайдер этой панели.",
    "providers.help": "Какие провайдеры вообще опрашиваются. Выключенный пропадает из трея, панели и отчётов, пока не включите снова; включение действует со следующего обновления. Сохранение ключа само включает провайдера.",
    "providers.bar_title": "Провайдеры в панели",
    "providers.bar_help": "Выберите записи для панели, включая именованные аккаунты. Изменения сохраняются сразу. Скрытые записи остаются в окне. Все встроенные провайдеры, включая неактивные, доступны в разделе опроса ниже.",
    "providers.shown": "Показан в панели.",
    "providers.hidden": "Скрыт в панели.",
    "providers.fetch_title": "Дополнительно: опрос провайдеров",
    "providers.none_shown": "AI Usage · в панели нет провайдеров",
    "credentials.help": "Значения ключей в shell не загружаются. Оставьте поле пустым, чтобы сохранить текущее, или очистите кнопкой. Переменные окружения имеют приоритет.",
    "credentials.keep_blank": "Оставьте пустым, чтобы сохранить текущий ключ",
    "credentials.paste": "Вставьте {label}",
    "credentials.keep_key": "Оставить сохранённый ключ",
    "credentials.clear_key": "Удалить ключ из конфига",
    "credentials.credential": "ключ",
    "credentials.new_key": "новый ключ",
    "credentials.env_override": "переопределение из окружения",
    "credentials.stored": "сохранено",
    "credentials.not_configured": "не настроено",
    "detail.used_of": "{used} из {limit} использовано ({percent}%)",
    "credentials.api_key": "API-ключ",
    "credentials.note.usage_quota": "квота использования",
    "error.binary_old": "Установленный ai-usagebar слишком старый для нативных настроек. Обновите пакет.",
    "error.apply": "Команда настроек не подтвердила сохранение.",
    "graphic.title": "Ожидаемое и фактическое использование",
    "graphic.expected": "Ожидаемое (линейно)",
    "graphic.weekly": "Неделя",
    "graphic.monthly": "Месяц",
    "tip.cached": "кэш",
    "ready": "Готово",
    "error": "Ошибка",
    "reset.due": "Пора сбросить",
    "reset.date": "{month} {day}",
    "reset.in": "Сброс через {duration} · {clock}",
    "updated.unavailable": "Время обновления недоступно",
    "updated.just_now": "Обновлено только что",
    "updated.ago": "Обновлено {duration} назад",
    "duration.now": "сейчас",
    "duration.days_hours": "{days}д {hours}ч",
    "duration.hours_minutes": "{hours}ч {minutes}м",
    "duration.minutes": "{minutes}м"
  },
  "pt-BR": {
    "app.name": "AI usage",
    "hero.settings": "Configurações",
    "hero.settings_meta": "Exibição, provedor e chaves de API",
    "hero.settings_detail": "A configuração atual permanece até você salvar.",
    "hero.usage_limits": "Uso e limites",
    "hero.loading": "Carregando provedores",
    "hero.usage_report": "Relatório de uso",
    "hero.provider_unavailable": "Provedor indisponível",
    "action.refresh": "Atualizar uso",
    "action.settings": "Configurações",
    "action.back": "Voltar ao uso",
    "action.retry": "Tentar novamente",
    "action.save": "Salvar configurações",
    "action.saving": "Salvando…",
    "section.usage": "USO",
    "section.usage_balance": "USO E SALDO",
    "section.settings": "CONFIGURAÇÕES",
    "section.display": "EXIBIÇÃO",
    "section.bar_window": "JANELA NA BARRA",
    "section.language": "IDIOMA",
    "section.primary": "PROVEDOR PRINCIPAL",
    "section.providers": "PROVEDORES",
    "section.credentials": "CREDENCIAIS",
    "loading.config": "Carregando configuração…",
    "loading.providers": "Coletando provedores configurados…",
    "empty.no_usage": "Nenhum provedor configurado informou uso.",
    "status.cached": "Dados em cache · o provedor não enviou uma resposta nova.",
    "status.refresh_failed": "Falha na atualização; mostrando o relatório anterior. {error}",
    "status.filter_miss": "Nenhuma entrada configurada corresponde a ‘{id}’. Limpe o provedor nas configurações ou use um id de ai-usagebar usage --json.",
    "status.saved": "Configurações salvas. Atualizando o uso.",
    "status.vendor_on": "Ativado — incluído no relatório.",
    "status.vendor_off": "Desativado — não consultado.",
    "status.will_clear": "será removido",
    "status.env_set": "definido no ambiente",
    "status.inline_set": "salvo no config",
    "status.not_set": "não definido",
    "toggle.show_value": "Mostrar valor de uso na barra",
    "toggle.show_value_desc": "Desative para deixar só o ícone. O painel e a dica ainda mostram os detalhes. Aplica imediatamente.",
    "toggle.show_provider": "Mostrar nome do provedor na barra",
    "toggle.show_provider_desc": "Prefixa a entrada da barra com o código curto do provedor — ocg, opr, dsk. Desativado por padrão. Aplica imediatamente.",
    "toggle.show_all": "Mostrar todos os provedores na barra",
    "toggle.show_all_desc": "Mostra ícone e uso de todos os provedores configurados de uma vez, em vez de alternar um por um. O clique ainda abre o painel; a roda ainda escolhe os detalhes. Desativado por padrão. Aplica imediatamente.",
    "toggle.color_code": "Colorir uso por nível",
    "toggle.color_code_desc": "Pinta valores da barra, medidores do painel e a dica de verde → amarelo → laranja → vermelho conforme o uso sobe, com as cores do tema Omarchy. Desative para uma só cor de texto. Desativado por padrão. Aplica imediatamente.",
    "bar_window.help": "Qual cota a barra mostra. Sem essa janela, usa a maior. Aplica imediatamente.",
    "bar_window.auto": "Maior uso (auto)",
    "bar_window.session": "5 horas (session)",
    "bar_window.weekly": "7 dias (weekly)",
    "bar_window.monthly": "Mensal (monthly)",
    "toggle.brand_icons": "Mostrar logos dos provedores",
    "toggle.brand_icons_desc": "Desenha a marca de cada provedor na barra e no painel. Desative para voltar ao ícone genérico de antes: um robô para um único provedor e o código curto do provedor em cada chip quando \"Mostrar todos os provedores na barra\" está ligado. Ativado por padrão. Aplica imediatamente.",
    // Notificações de limite de cota ([notifications] no config.toml).
    "section.notifications": "NOTIFICAÇÕES",
    "notify.enabled": "Notificar quando uma janela de cota se aproxima do limite",
    "notify.enabled_desc": "Envia uma notificação quando uma janela cruza o limite abaixo e avisa 48 horas antes de os créditos acumulados expirarem. Uma notificação por cruzamento; só volta a disparar após queda de 7 pontos.",
    "notify.threshold": "Limite (%)",
    "notify.threshold_desc": "Um número inteiro de 1 a 100. Em 100, apenas uma janela já esgotada notifica.",
    "notify.threshold_invalid": "Informe um número inteiro de 1 a 100.",
    "status.notify_off": "desligado",
    "error.notify_threshold": "O limite de notificação deve ser um número inteiro entre 1 e 100.",
    "section.metrics": "MÉTRICAS",
    "metrics.help": "Escolha quais métricas cada provedor mostra. Uma métrica desligada some do painel, da barra e da dica, e é ignorada quando a barra escolhe a maior porcentagem. A última métrica ligada não pode ser desligada. Aplica imediatamente.",
    "section.show_as": "EXIBIR USO COMO",
    "show_as.help": "Se as porcentagens mostram o que você já usou ou o que resta da mesma janela. Aplica imediatamente.",
    "show_as.used": "Usado",
    "show_as.left": "Restante",
    "monthly_cycle_day": "Dia de reinício do ciclo mensal",
    "monthly_cycle_day_desc": "Dia do mês (1-28) em que o ciclo de cobrança do OpenCode Go é reiniciado. Se não definido, estima-se um ciclo de 30 dias.",
    "monthly_cycle_day_unset": "Não definido",
    "metric.left": "{percent}% restante",
    "language.help": "Idioma do painel e das configurações. Automático segue o idioma do sistema.",
    "language.auto": "Sistema (auto)",
    "language.en": "English",
    "language.ru": "Русский",
    "language.pt-BR": "Português (Brasil)",
    "language.ko": "한국어",
    "language.es": "Español",
    "primary.help": "Usado pelo CLI e como provedor preferido deste painel.",
    "providers.help": "Quais provedores são consultados. Desligar remove da barra, do painel e dos relatórios até ligar de novo; ligar vale na próxima atualização. Salvar uma credencial mantém o provedor ligado.",
    "providers.bar_title": "Provedores na barra",
    "providers.bar_help": "Escolha quais entradas aparecem na barra, incluindo contas nomeadas. As alterações são salvas imediatamente. Entradas ocultas continuam no painel. Todos os provedores integrados, inclusive os inativos, ficam na seção avançada de consultas.",
    "providers.shown": "Visível na barra.",
    "providers.hidden": "Oculto na barra.",
    "providers.fetch_title": "Avançado: consulta de provedores",
    "providers.none_shown": "AI Usage · nenhum provedor visível na barra",
    "credentials.help": "Valores salvos nunca são carregados no shell. Deixe em branco para manter o atual, ou use limpar para remover a credencial inline. Variáveis de ambiente têm prioridade.",
    "credentials.keep_blank": "Deixe em branco para manter a credencial atual",
    "credentials.paste": "Cole {label}",
    "credentials.keep_key": "Manter a chave salva",
    "credentials.clear_key": "Limpar a chave salva no config",
    "credentials.credential": "credencial",
    "credentials.new_key": "nova chave",
    "credentials.env_override": "sobrescrita do ambiente",
    "credentials.stored": "salvo",
    "credentials.not_configured": "não configurado",
    "detail.used_of": "{used} de {limit} usados ({percent}%)",
    "credentials.api_key": "Chave de API",
    "credentials.note.usage_quota": "cota de uso",
    "error.binary_old": "O binário ai-usagebar instalado é anterior às configurações nativas. Atualize o pacote.",
    "error.apply": "O comando de configurações não confirmou o salvamento.",
    "graphic.title": "Uso esperado vs real",
    "graphic.expected": "Esperado (linear)",
    "graphic.weekly": "Semanal",
    "graphic.monthly": "Mensal",
    "tip.cached": "em cache",
    "ready": "Pronto",
    "error": "Erro",
    "reset.due": "Redefinição devida",
    "reset.date": "{month} {day}",
    "reset.in": "Redefine em {duration} · {clock}",
    "updated.unavailable": "Horário de atualização indisponível",
    "updated.just_now": "Atualizado agora",
    "updated.ago": "Atualizado há {duration}",
    "duration.now": "agora",
    "duration.days_hours": "{days}d {hours}h",
    "duration.hours_minutes": "{hours}h {minutes}m",
    "duration.minutes": "{minutes}m"
  },
  ko: {
    "app.name": "AI 사용량",
    "hero.settings": "설정",
    "hero.settings_meta": "표시, 제공자, API 키",
    "hero.settings_detail": "저장하기 전까지 기존 설정은 그대로 유지됩니다.",
    "hero.usage_limits": "사용량과 한도",
    "hero.loading": "제공자 불러오는 중",
    "hero.usage_report": "사용량 보고서",
    "hero.provider_unavailable": "제공자를 사용할 수 없음",
    "action.refresh": "사용량 새로 고침",
    "action.settings": "설정",
    "action.back": "사용량으로 돌아가기",
    "action.retry": "다시 시도",
    "action.save": "설정 저장",
    "action.saving": "저장 중…",
    "metric.left": "{percent}% 남음",
    "metrics.help": "각 공급자가 표시할 지표를 선택하세요. 끈 지표는 패널, 상단 바, 툴팁에서 사라지고, 최고 백분율을 고를 때 무시됩니다. 마지막으로 켜진 지표는 끌 수 없습니다. 즉시 적용됩니다.",
    "graphic.title": "예상 대비 실제 사용량",
    "graphic.expected": "예상 (선형)",
    "graphic.weekly": "주간",
    "graphic.monthly": "월간",
    "section.metrics": "지표",
    "section.show_as": "사용량 표시",
    "section.usage": "사용량",
    "section.usage_balance": "사용량 및 잔액",
    "section.settings": "설정",
    "section.display": "표시",
    "section.bar_window": "상단 바 기간",
    "section.language": "언어",
    "section.primary": "기본 제공자",
    "section.providers": "제공자",
    "section.credentials": "자격 증명",
    "loading.config": "설정 불러오는 중…",
    "loading.providers": "설정된 제공자를 모으는 중…",
    "empty.no_usage": "사용량을 보고한 제공자가 없습니다.",
    "show_as.help": "백분율을 같은 기간에서 사용한 양으로 읽을지 남은 양으로 읽을지 선택합니다. 즉시 적용됩니다.",
    "show_as.left": "남음",
    "show_as.used": "사용함",
    "monthly_cycle_day": "월간 주기 재설정 날짜",
    "monthly_cycle_day_desc": "OpenCode Go 청구 주기가 재설정되는 월의 날짜(1-28)입니다. 설정하지 않으면 30일 주기로 추정합니다.",
    "monthly_cycle_day_unset": "설정 안 함",
    "status.cached": "캐시된 데이터 · 제공자가 새 응답을 주지 못했습니다.",
    "status.refresh_failed": "새로 고치지 못해 이전 보고서를 표시합니다. {error}",
    "status.filter_miss": "‘{id}’와 일치하는 항목이 없습니다. 제공자 설정을 비우거나 ai-usagebar usage --json의 id를 사용하세요.",
    "status.saved": "설정을 저장했습니다. 사용량을 새로 고치는 중입니다.",
    "status.vendor_on": "켬 — 보고서에 포함합니다.",
    "status.vendor_off": "끔 — 가져오지 않습니다.",
    "status.will_clear": "삭제 예정",
    "status.env_set": "환경 변수에 설정됨",
    "status.inline_set": "설정 파일에 저장됨",
    "status.not_set": "설정 안 됨",
    "toggle.brand_icons": "공급자 로고 표시",
    "toggle.brand_icons_desc": "상단 바와 패널에 각 공급자의 고유 마크를 표시합니다. 끄면 이전의 일반 아이콘으로 돌아갑니다: 공급자가 하나일 때는 로봇 아이콘, \"상단 바에 모든 공급자 표시\"가 켜져 있을 때는 각 칩에 공급자의 짧은 코드를 표시합니다. 기본적으로 켜져 있습니다. 즉시 적용됩니다.",
    // 할당량 임계값 알림 (config.toml의 [notifications]).
    "section.notifications": "알림",
    "notify.enabled": "할당량 한도에 가까워지면 알림",
    "notify.enabled_desc": "아래 임계값을 창이 넘으면 알림을 한 번 보내고, 적립된 크레딧이 만료되기 48시간 전에 경고합니다. 한 번 넘을 때 한 번만 알리며, 7포인트 떨어져야 다시 무장됩니다.",
    "notify.threshold": "임계값 (%)",
    "notify.threshold_desc": "1에서 100 사이의 정수입니다. 100이면 이미 소진된 창만 알립니다.",
    "notify.threshold_invalid": "1에서 100 사이의 정수를 입력하세요.",
    "status.notify_off": "꺼짐",
    "error.notify_threshold": "알림 임계값은 1에서 100 사이의 정수여야 합니다.",
    "toggle.show_value": "상단 바에 사용량 값 표시",
    "toggle.show_value_desc": "끄면 상단 바에 아이콘만 표시합니다. 패널과 툴팁에는 전체 사용량이 그대로 나옵니다. 즉시 적용됩니다.",
    "toggle.show_provider": "상단 바에 제공자 이름 표시",
    "toggle.show_provider_desc": "켜면 상단 바 항목 앞에 제공자 약칭(ocg, opr, dsk)을 붙입니다. 기본값은 끔입니다. 즉시 적용됩니다.",
    "toggle.show_all": "상단 바에 모든 제공자 표시",
    "toggle.show_all_desc": "켜면 하나씩 순환하지 않고 설정된 모든 제공자의 아이콘과 사용량을 상단 바에 한꺼번에 표시합니다. 클릭하면 패널이 열리고, 휠로 볼 항목을 고릅니다. 기본값은 끔입니다. 즉시 적용됩니다.",
    "toggle.color_code": "사용량 수준별 색상 표시",
    "toggle.color_code_desc": "사용량이 오르면 Omarchy 테마 색으로 바 값, 패널 미터, 툴팁을 초록 → 노랑 → 주황 → 빨강으로 칠합니다. 끄면 모두 한 가지 전경색을 씁니다. 기본값은 끔입니다. 즉시 적용됩니다.",
    "bar_window.help": "상단 바에 표시할 할당량입니다. 해당 기간이 없는 제공자는 최고값으로 대체합니다. 즉시 적용됩니다.",
    "bar_window.auto": "최고값(자동)",
    "bar_window.session": "5시간(세션)",
    "bar_window.weekly": "7일(주간)",
    "bar_window.monthly": "월간",
    "language.help": "패널과 설정에 쓸 언어입니다. 자동은 시스템 언어를 따릅니다.",
    "language.auto": "시스템(자동)",
    "language.en": "English",
    "language.ru": "Русский",
    "language.pt-BR": "Português (Brasil)",
    "language.ko": "한국어",
    "language.es": "Español",
    "primary.help": "CLI에서 사용하고 이 패널에서 우선 표시하는 제공자입니다.",
    "providers.help": "가져올 제공자를 고릅니다. 끈 제공자는 다시 켤 때까지 바, 패널, 보고서에서 빠지고, 켠 제공자는 다음 새로 고침부터 반영됩니다. 제공자의 자격 증명을 저장하면 그 제공자는 켜진 상태로 유지됩니다.",
    "providers.bar_title": "바에 표시할 제공자",
    "providers.bar_help": "이름이 있는 계정을 포함하여 바에 표시할 항목을 선택합니다. 변경은 즉시 저장되고 숨긴 항목도 패널에 남습니다. 비활성 제공자를 포함한 모든 기본 제공자는 아래 고급 가져오기 설정에 있습니다.",
    "providers.shown": "바에 표시됩니다.",
    "providers.hidden": "바에서 숨겨집니다.",
    "providers.fetch_title": "고급: 제공자 데이터 가져오기",
    "providers.none_shown": "AI Usage · 바에 표시된 제공자가 없습니다",
    "credentials.help": "저장된 값은 셸에 불러오지 않습니다. 현재 값을 유지하려면 비워 두고, 설정 파일의 자격 증명을 지우려면 삭제 버튼을 누르세요. 환경 변수가 우선합니다.",
    "credentials.keep_blank": "현재 자격 증명을 유지하려면 비워 두세요",
    "credentials.paste": "{label} 붙여넣기",
    "credentials.keep_key": "저장된 키 유지",
    "credentials.clear_key": "설정 파일에 저장된 키 삭제",
    "credentials.credential": "자격 증명",
    "credentials.new_key": "새 키",
    "credentials.env_override": "환경 변수 우선",
    "credentials.stored": "저장됨",
    "credentials.not_configured": "설정 안 됨",
    "detail.used_of": "{limit} 중 {used} 사용({percent}%)",
    "credentials.api_key": "API 키",
    "credentials.note.usage_quota": "사용 할당량",
    "error.binary_old": "설치된 ai-usagebar 바이너리가 네이티브 설정보다 오래된 버전입니다. 패키지를 업데이트하세요.",
    "error.apply": "설정 명령이 저장을 확인하지 않았습니다.",
    "tip.cached": "캐시됨",
    "ready": "준비됨",
    "error": "오류",
    "reset.due": "초기화 예정",
    "reset.date": "{month} {day}일",
    "reset.in": "{duration} 후 초기화 · {clock}",
    "updated.unavailable": "업데이트 시각 없음",
    "updated.just_now": "방금 업데이트됨",
    "updated.ago": "{duration} 전 업데이트됨",
    "duration.now": "지금",
    "duration.days_hours": "{days}일 {hours}시간",
    "duration.hours_minutes": "{hours}시간 {minutes}분",
    "duration.minutes": "{minutes}분"
  },
  es: {
    "app.name": "Uso de IA",
    "hero.settings": "Ajustes",
    "hero.settings_meta": "Pantalla, proveedor y claves de API",
    "hero.settings_detail": "La configuración actual se mantiene hasta que guardes.",
    "hero.usage_limits": "Uso y límites",
    "hero.loading": "Cargando proveedores",
    "hero.usage_report": "Informe de uso",
    "hero.provider_unavailable": "Proveedor no disponible",
    "action.refresh": "Actualizar uso",
    "action.settings": "Ajustes",
    "action.back": "Volver al uso",
    "action.retry": "Reintentar",
    "action.save": "Guardar ajustes",
    "action.saving": "Guardando…",
    "section.usage": "USO",
    "section.usage_balance": "USO Y SALDO",
    "section.settings": "AJUSTES",
    "section.display": "PANTALLA",
    "section.bar_window": "VENTANA EN LA BARRA",
    "section.language": "IDIOMA",
    "section.primary": "PROVEEDOR PRINCIPAL",
    "section.providers": "PROVEEDORES",
    "section.credentials": "CREDENCIALES",
    "loading.config": "Cargando configuración…",
    "loading.providers": "Reuniendo los proveedores configurados…",
    "empty.no_usage": "Ningún proveedor configurado informó uso.",
    "status.cached": "Datos en caché · el proveedor no pudo enviar una respuesta nueva.",
    "status.refresh_failed": "No se pudo actualizar; se muestra el informe anterior. {error}",
    "status.filter_miss": "Ninguna entrada configurada coincide con ‘{id}’. Borra el proveedor en los ajustes o usa un id de ai-usagebar usage --json.",
    "status.saved": "Ajustes guardados. Actualizando el uso.",
    "status.vendor_on": "Activado — incluido en el informe.",
    "status.vendor_off": "Desactivado — no se consulta.",
    "status.will_clear": "se borrará",
    "status.env_set": "definido en el entorno",
    "status.inline_set": "guardado en la configuración",
    "status.not_set": "sin definir",
    "toggle.show_value": "Mostrar el uso en la barra superior",
    "toggle.show_value_desc": "Desactívalo para dejar solo el ícono en la barra. El panel y la descripción emergente siguen mostrando todos los detalles. Se aplica de inmediato.",
    "toggle.show_provider": "Mostrar el nombre del proveedor en la barra superior",
    "toggle.show_provider_desc": "Antepone a la entrada de la barra el código corto del proveedor — ocg, opr, dsk. Desactivado por defecto. Se aplica de inmediato.",
    "toggle.show_all": "Mostrar todos los proveedores en la barra superior",
    "toggle.show_all_desc": "Muestra a la vez el ícono y el uso de cada proveedor configurado, en lugar de alternar uno por uno. El clic sigue abriendo el panel; la rueda sigue eligiendo qué detalles ves. Desactivado por defecto. Se aplica de inmediato.",
    "toggle.color_code": "Colorear el uso por nivel",
    "toggle.color_code_desc": "Pinta los valores de la barra, los medidores del panel y la descripción emergente de verde → amarillo → naranja → rojo a medida que sube el uso, con los colores de tu tema de Omarchy. Desactívalo para usar un solo color en todo. Desactivado por defecto. Se aplica de inmediato.",
    "bar_window.help": "Qué cuota muestra la barra. Los proveedores que no la tienen usan la más alta. Se aplica de inmediato.",
    "bar_window.auto": "Mayor uso (auto)",
    "bar_window.session": "5 horas (session)",
    "bar_window.weekly": "7 días (weekly)",
    "bar_window.monthly": "Mensual (monthly)",
    "toggle.brand_icons": "Mostrar logos de los proveedores",
    "toggle.brand_icons_desc": "Dibuja la marca de cada proveedor en la barra superior y en el panel. Desactívalo para volver al ícono genérico de antes: un robot para un solo proveedor y el código corto del proveedor en cada chip cuando \"Mostrar todos los proveedores en la barra superior\" está activado. Activado por defecto. Se aplica de inmediato.",
    // Notificaciones de umbral de cuota ([notifications] en config.toml).
    "section.notifications": "NOTIFICACIONES",
    "notify.enabled": "Notificar cuando una ventana de cuota se acerque al límite",
    "notify.enabled_desc": "Envía una notificación cuando una ventana cruza el umbral de abajo y avisa 48 horas antes de que expiren los créditos acumulados. Una notificación por cruce; vuelve a dispararse solo tras bajar 7 puntos.",
    "notify.threshold": "Umbral (%)",
    "notify.threshold_desc": "Un número entero de 1 a 100. Con 100 solo notifica una ventana ya agotada.",
    "notify.threshold_invalid": "Introduce un número entero de 1 a 100.",
    "status.notify_off": "desactivado",
    "error.notify_threshold": "El umbral de notificación debe ser un número entero entre 1 y 100.",
    "section.metrics": "MÉTRICAS",
    "metrics.help": "Elige qué métricas muestra cada proveedor. Una métrica desactivada desaparece del panel, la barra superior y la descripción emergente, y se ignora cuando la barra elige el porcentaje más alto. La última métrica activa no se puede desactivar. Se aplica de inmediato.",
    "section.show_as": "MOSTRAR USO COMO",
    "show_as.help": "Si los porcentajes indican lo que ya usaste o lo que queda de la misma ventana. Se aplica de inmediato.",
    "show_as.used": "Usado",
    "show_as.left": "Restante",
    "monthly_cycle_day": "Día de reinicio del ciclo mensual",
    "monthly_cycle_day_desc": "Día del mes (1-28) en que se reinicia el ciclo de facturación de OpenCode Go. Si no se define, se estima un ciclo de 30 días.",
    "monthly_cycle_day_unset": "No definido",
    "metric.left": "{percent}% restante",
    "language.help": "Idioma del panel y de los ajustes. Automático sigue el idioma del sistema.",
    "language.auto": "Sistema (auto)",
    "language.en": "English",
    "language.ru": "Русский",
    "language.pt-BR": "Português (Brasil)",
    "language.ko": "한국어",
    "language.es": "Español",
    "primary.help": "Lo usa la CLI y es el proveedor preferido de este panel.",
    "providers.help": "Qué proveedores se consultan. Si desactivas uno, sale de la barra, el panel y los informes hasta que lo vuelvas a activar; activar uno surte efecto en la próxima actualización. Guardar una credencial de un proveedor lo mantiene activado.",
    "providers.bar_title": "Proveedores en la barra",
    "providers.bar_help": "Elige qué entradas aparecen en la barra, incluidas las cuentas con nombre. Los cambios se guardan al instante. Las entradas ocultas siguen en el panel. Todos los proveedores integrados, incluidos los inactivos, están en la sección avanzada de consultas.",
    "providers.shown": "Visible en la barra.",
    "providers.hidden": "Oculto en la barra.",
    "providers.fetch_title": "Avanzado: consulta de proveedores",
    "providers.none_shown": "AI Usage · ningún proveedor visible en la barra",
    "credentials.help": "Los valores guardados nunca se cargan en el shell. Deja un campo en blanco para conservar su valor actual, o usa el botón de borrar para quitar una credencial guardada en la configuración. Las variables de entorno tienen prioridad.",
    "credentials.keep_blank": "Déjalo en blanco para conservar la credencial actual",
    "credentials.paste": "Pega {label}",
    "credentials.keep_key": "Conservar la clave guardada",
    "credentials.clear_key": "Borrar la clave guardada en la configuración",
    "credentials.credential": "credencial",
    "credentials.new_key": "clave nueva",
    "credentials.env_override": "definida en el entorno",
    "credentials.stored": "guardada",
    "credentials.not_configured": "sin configurar",
    "detail.used_of": "{used} de {limit} usados ({percent}%)",
    "credentials.api_key": "Clave de API",
    "credentials.note.usage_quota": "cuota de uso",
    "error.binary_old": "El binario de ai-usagebar instalado es anterior a los ajustes nativos. Actualiza el paquete.",
    "error.apply": "El comando de ajustes no confirmó que se guardaran.",
    "graphic.title": "Uso esperado vs real",
    "graphic.expected": "Esperado (lineal)",
    "graphic.weekly": "Semanal",
    "graphic.monthly": "Mensual",
    "tip.cached": "en caché",
    "ready": "Listo",
    "error": "Error",
    "reset.due": "Reinicio pendiente",
    "reset.date": "{day} {month}",
    "reset.in": "Se reinicia en {duration} · {clock}",
    "updated.unavailable": "Hora de actualización no disponible",
    "updated.just_now": "Actualizado ahora mismo",
    "updated.ago": "Actualizado hace {duration}",
    "duration.now": "ahora",
    "duration.days_hours": "{days}d {hours}h",
    "duration.hours_minutes": "{hours}h {minutes}m",
    "duration.minutes": "{minutes}m"
  }
}

function normalizeLocaleTag(value) {
  var text = String(value === undefined || value === null ? "" : value).trim().toLowerCase()
  if (text === "auto" || text === "") return "auto"
  if (text.indexOf("pt") === 0) return "pt-BR"
  if (text.indexOf("ru") === 0) return "ru"
  if (text.indexOf("en") === 0) return "en"
  if (text.indexOf("ko") === 0) return "ko"
  if (text.indexOf("es") === 0) return "es"
  var dash = text.indexOf("-")
  var under = text.indexOf("_")
  var cut = dash >= 0 ? dash : under
  var base = cut >= 0 ? text.slice(0, cut) : text
  if (SUPPORTED.indexOf(base) >= 0) return base
  return ""
}

function resolveLocale(setting, systemName) {
  var chosen = normalizeLocaleTag(setting)
  if (chosen && chosen !== "auto") return chosen
  var system = normalizeLocaleTag(systemName)
  if (system && system !== "auto") return system
  return DEFAULT_LOCALE
}

function catalog(locale) {
  var tag = resolveLocale(locale, "")
  return MESSAGES[tag] || MESSAGES[DEFAULT_LOCALE]
}

function t(locale, key, params) {
  var table = catalog(locale)
  var fallback = MESSAGES[DEFAULT_LOCALE]
  var template = table[key]
  if (template === undefined || template === null) template = fallback[key]
  if (template === undefined || template === null) return String(key || "")
  var out = String(template)
  var values = params && typeof params === "object" ? params : {}
  for (var name in values) {
    if (!Object.prototype.hasOwnProperty.call(values, name)) continue
    out = out.split("{" + name + "}").join(String(values[name]))
  }
  return out
}

function displayDetail(locale, detail) {
  var text = String(detail || "").trim()
  if (text === "") return ""
  var used = text.match(/^(.+?) of (.+?) used \((\d+)%\)$/)
  if (used)
    return t(locale, "detail.used_of", { used: used[1], limit: used[2], percent: used[3] })
  return text
}

function displaySecretLabel(locale, label) {
  var text = String(label || "").trim()
  if (text === "API key") return t(locale, "credentials.api_key")
  return text
}

function displayNote(locale, note) {
  var text = String(note || "").trim()
  if (text === "") return ""
  var key = NOTE_KEYS[text]
  if (key) return t(locale, key)
  return text
}

function monthName(locale, monthIndex) {
  var tag = resolveLocale(locale, "")
  var list = MONTHS[tag] || MONTHS[DEFAULT_LOCALE]
  var idx = Number(monthIndex)
  if (!(idx >= 0 && idx < list.length)) return ""
  return list[idx]
}

function formatDuration(milliseconds, locale) {
  if (!(milliseconds > 0)) return t(locale, "duration.now")
  var minutes = Math.floor(milliseconds / 60000)
  var hours = Math.floor(minutes / 60)
  var days = Math.floor(hours / 24)
  if (days > 0)
    return t(locale, "duration.days_hours", { days: days, hours: hours % 24 })
  if (hours > 0)
    return t(locale, "duration.hours_minutes", { hours: hours, minutes: minutes % 60 })
  return t(locale, "duration.minutes", { minutes: Math.max(1, minutes) })
}

function pad2(value) {
  return ("0" + value).slice(-2)
}

function isSameLocalDay(a, b) {
  return a.getFullYear() === b.getFullYear()
    && a.getMonth() === b.getMonth()
    && a.getDate() === b.getDate()
}

// Locale-aware copies of Model.formatReset / formatUpdated for panel display.
function formatReset(resetAt, nowMs, locale) {
  if (!resetAt) return ""
  var resetMs = new Date(String(resetAt)).getTime()
  if (!isFinite(resetMs)) return ""
  var remaining = resetMs - Number(nowMs)
  if (remaining <= 0) return t(locale, "reset.due")
  var at = new Date(resetMs)
  var clock = pad2(at.getHours()) + ":" + pad2(at.getMinutes())
  if (!isSameLocalDay(at, new Date(Number(nowMs))))
    clock = t(locale, "reset.date", { month: monthName(locale, at.getMonth()), day: at.getDate() }) + " " + clock
  return t(locale, "reset.in", {
    duration: formatDuration(remaining, locale),
    clock: clock
  })
}

function formatUpdated(fetchedAt, nowMs, locale) {
  if (!fetchedAt) return t(locale, "updated.unavailable")
  var fetchedMs = new Date(String(fetchedAt)).getTime()
  if (!isFinite(fetchedMs)) return t(locale, "updated.unavailable")
  var elapsed = Math.max(0, Number(nowMs) - fetchedMs)
  if (elapsed < 60000) return t(locale, "updated.just_now")
  return t(locale, "updated.ago", { duration: formatDuration(elapsed, locale) })
}
