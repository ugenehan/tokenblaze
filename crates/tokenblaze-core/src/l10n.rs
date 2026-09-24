//! 🌍 Localization.
//!
//! Hardcoded translation tables — only 4 languages, no need for fluent.
//! Faster, simpler, and doesn't depend on runtime locale detection.

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppLanguage {
    System,
    #[default]
    English,
    Chinese,
    Japanese,
    Korean,
}

impl AppLanguage {
    pub fn all() -> [AppLanguage; 5] {
        use AppLanguage::*;
        [System, English, Chinese, Japanese, Korean]
    }

    pub fn label(self) -> &'static str {
        match self {
            AppLanguage::System => "System",
            AppLanguage::English => "English",
            AppLanguage::Chinese => "中文",
            AppLanguage::Japanese => "日本語",
            AppLanguage::Korean => "한국어",
        }
    }

    /// Effective language (resolves System to actual language).
    pub fn effective(self) -> AppLanguage {
        match self {
            AppLanguage::System => detect_system_language(),
            other => other,
        }
    }
}

fn detect_system_language() -> AppLanguage {
    #[cfg(target_os = "windows")]
    if let Some(language) = windows_system_language() {
        return language;
    }

    // Unix desktops normally expose the locale through LANG.
    if let Ok(lang) = std::env::var("LANG") {
        if let Some(language) = language_from_locale(&lang) {
            return language;
        }
    }
    AppLanguage::English
}

fn language_from_locale(locale: &str) -> Option<AppLanguage> {
    let locale = locale.to_ascii_lowercase();
    if locale.starts_with("zh") {
        Some(AppLanguage::Chinese)
    } else if locale.starts_with("ja") {
        Some(AppLanguage::Japanese)
    } else if locale.starts_with("ko") {
        Some(AppLanguage::Korean)
    } else if locale.starts_with("en") {
        Some(AppLanguage::English)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn windows_system_language() -> Option<AppLanguage> {
    use winapi::um::winnls::GetUserDefaultLocaleName;

    // LOCALE_NAME_MAX_LENGTH, including the trailing NUL.
    let mut locale = [0u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(locale.as_mut_ptr(), locale.len() as i32) };
    if length <= 1 {
        return None;
    }
    let locale = String::from_utf16_lossy(&locale[..length as usize - 1]);
    language_from_locale(&locale)
}

/// Look up a translated string.
pub fn t(key: &'static str, lang: AppLanguage) -> &'static str {
    let table = match lang.effective() {
        AppLanguage::Chinese => ZH,
        AppLanguage::Japanese => JA,
        AppLanguage::Korean => KO,
        AppLanguage::English | AppLanguage::System => EN,
    };
    lookup(table, key).unwrap_or_else(|| {
        // Fall back to English
        lookup(EN, key).unwrap_or(key)
    })
}

// ── English (default) ───────────────────────────────────────────

const EN: &[(&str, &str)] = &[
    ("app.name", "TokenBlaze"),
    (
        "app.tagline",
        "If you're burning tokens anyway, light a real fire.",
    ),
    ("console.title", "Console"),
    ("console.productSubtitle", "Local AI usage hearth"),
    ("nav.overview", "OVERVIEW"),
    ("nav.todayFire", "Today's fire"),
    ("nav.manage", "MANAGE"),
    ("nav.appearance", "Flame appearance"),
    ("nav.preferences", "Preferences"),
    ("nav.burningNormally", "Burning normally"),
    (
        "console.footnote",
        "Local usage logs only — nothing is uploaded.",
    ),
    ("console.version", "Version %@"),
    ("console.website", "Website"),
    ("console.contact", "Contact developer"),
    ("debug.title", "Debug"),
    (
        "debug.hint",
        "Force flame look. Closing this window hides debug again.",
    ),
    ("debug.live", "Live"),
    ("debug.inject", "Inject tokens"),
    ("debug.autoBurn", "Auto burn"),
    ("debug.pause", "Pause animation"),
    ("debug.intensity", "Force intensity"),
    ("debug.applyIntensity", "Apply custom intensity"),
    ("language", "Language"),
    ("language.system", "System"),
    ("language.english", "English"),
    ("language.chinese", "中文"),
    ("language.japanese", "日本語"),
    ("language.korean", "한국어"),
    ("sources.title", "Sources"),
    ("sources.rescan", "Rescan"),
    ("sources.detail", "Details"),
    ("sources.lastRead", "Last read"),
    ("sources.notRead", "Not read yet"),
    ("sources.recent", "Recent events"),
    ("sources.noRecent", "No recent events"),
    ("sources.estimated", "estimated"),
    ("source.state.ok", "Connected"),
    ("source.state.notFound", "Not found"),
    ("source.state.noPermission", "No permission"),
    ("source.state.unsupported", "Unsupported"),
    ("source.state.readError", "Read error"),
    ("source.claude_code", "Claude Code"),
    ("source.codex", "Codex"),
    ("source.cursor", "Cursor"),
    ("source.grok", "Grok"),
    ("source.pi", "Pi"),
    ("source.amp", "Amp"),
    ("source.opencode", "OpenCode"),
    ("colors.title", "Flame colors"),
    ("colors.reset", "Reset"),
    (
        "colors.mix.empty",
        "Classic orange — waiting for live usage",
    ),
    ("colors.mix.active", "Live mix"),
    ("stats.title", "Today"),
    ("stats.todayTokens", "TODAY'S TOKENS"),
    ("stats.currentState", "Current state"),
    ("stats.sourceMix", "Source mix"),
    ("stats.fuelStatus", "Fuel status"),
    ("stats.fuelRemaining", "fuel remaining"),
    ("stats.emberHeat", "Ember heat"),
    ("stats.intensity", "Intensity"),
    ("stats.total", "Total"),
    ("stats.bySource", "By tool"),
    ("stats.hourly", "Through the day"),
    ("stats.breakdown", "Where tokens went"),
    ("stats.input", "Input"),
    ("stats.output", "Output"),
    ("stats.cacheRead", "Cache read"),
    ("stats.cacheWrite", "Cache write"),
    ("stats.other", "Other / estimated"),
    ("stats.empty", "No usage recorded yet today."),
    ("stats.tier", "Flame"),
    ("size.title", "Flame size"),
    ("size.small", "S"),
    ("size.medium", "M"),
    ("size.large", "L"),
    (
        "size.hint",
        "Scales the desktop campfire, not just the window.",
    ),
    ("sound.title", "Hearth sound"),
    ("sound.enabled", "On"),
    ("sound.volume", "Volume"),
    ("hover.settings.title", "Hover card"),
    ("menu.hideFlame", "Hide Flame"),
    ("menu.showFlame", "Show Flame"),
    ("menu.toggleFlame", "Show / Hide Flame"),
    ("menu.resetPosition", "Reset Position"),
    ("menu.openConsole", "Open Console"),
    ("menu.checkUpdates", "Check for Updates…"),
    ("menu.pauseAnimation", "Pause Animation"),
    ("menu.resumeAnimation", "Resume Animation"),
    ("menu.togglePause", "Pause / Resume Animation"),
    ("menu.quit", "Quit TokenBlaze"),
    ("settings.reduceMotion", "Reduce motion"),
    ("settings.showLiveRate", "Show live token rate"),
    (
        "settings.showLiveRate.hint",
        "Estimate only — based on recent local usage spikes, may not be exact.",
    ),
    (
        "settings.privacy",
        "Reads local Codex, Claude Code, Cursor, Grok, Pi, Amp, and OpenCode usage. Nothing is uploaded.",
    ),
    ("tier.hush", "Hush"),
    ("tier.glow", "Glow"),
    ("tier.crackle", "Crackle"),
    ("tier.roar", "Roar"),
    ("tier.blaze", "Blaze"),
    ("phase.unlit", "Unlit"),
    ("phase.flame", "Flame"),
    ("phase.ember", "Ember"),
    ("phase.out", "Out"),
    ("hover.today", "today tokens"),
    ("hover.rate", "token/s · est."),
    ("hover.estimated", "est."),
    ("hover.updated", "Updated %@"),
    ("update.title", "Update Available"),
    ("update.later", "Later"),
    ("update.download", "Download"),
    ("update.checking", "Checking for updates…"),
    ("update.upToDate", "TokenBlaze is up to date."),
    ("update.checkAgain", "Check again"),
    ("update.available", "Update available:"),
    (
        "update.noWindowsPackage",
        "A newer release exists, but it has no package for this platform:",
    ),
    ("update.openReleases", "Open releases"),
    ("update.downloading", "Downloading"),
    ("update.ready", "Ready to install:"),
    ("update.installRestart", "Install and restart"),
    ("update.failed", "Update failed"),
    ("update.retry", "Retry"),
];

// ── Chinese ────────────────────────────────────────────────────

const ZH: &[(&str, &str)] = &[
    ("app.name", "TokenBlaze"),
    ("app.tagline", "既然都在烧 token，不如真的生一把火。"),
    ("console.title", "面板"),
    ("console.productSubtitle", "本地 AI 用量壁炉"),
    ("nav.overview", "概览"),
    ("nav.todayFire", "今日火势"),
    ("nav.manage", "管理"),
    ("nav.appearance", "火焰外观"),
    ("nav.preferences", "偏好设置"),
    ("nav.burningNormally", "正常燃烧"),
    ("console.footnote", "仅读取本机用量日志，不上传。"),
    ("console.version", "版本 %@"),
    ("console.website", "网站"),
    ("console.contact", "联系开发者"),
    ("debug.title", "调试"),
    (
        "debug.hint",
        "强制预览火焰外观。关闭此窗口后会再次隐藏调试项。",
    ),
    ("debug.live", "实时"),
    ("debug.inject", "注入 token"),
    ("debug.autoBurn", "自动燃烧"),
    ("debug.pause", "暂停动画"),
    ("debug.intensity", "强制强度"),
    ("language", "语言"),
    ("language.system", "跟随系统"),
    ("language.english", "English"),
    ("language.chinese", "中文"),
    ("language.japanese", "日本語"),
    ("language.korean", "한국어"),
    ("sources.title", "数据源"),
    ("sources.rescan", "重新扫描"),
    ("sources.detail", "详情"),
    ("sources.lastRead", "最后读取"),
    ("sources.notRead", "尚未读取"),
    ("sources.recent", "最近事件"),
    ("sources.noRecent", "暂无最近事件"),
    ("sources.estimated", "估算"),
    ("source.state.ok", "正常"),
    ("source.state.notFound", "未发现"),
    ("source.state.noPermission", "无权限"),
    ("source.state.unsupported", "格式不支持"),
    ("source.state.readError", "读取异常"),
    ("source.claude_code", "Claude Code"),
    ("source.codex", "Codex"),
    ("source.cursor", "Cursor"),
    ("source.grok", "Grok"),
    ("source.pi", "Pi"),
    ("source.amp", "Amp"),
    ("source.opencode", "OpenCode"),
    ("colors.title", "火焰颜色"),
    ("colors.reset", "恢复默认"),
    ("colors.mix.empty", "经典橙火 — 尚无实时比例"),
    ("colors.mix.active", "实时燃烧比例"),
    ("stats.title", "今日"),
    ("stats.todayTokens", "今日 TOKENS"),
    ("stats.currentState", "当前状态"),
    ("stats.sourceMix", "来源构成"),
    ("stats.fuelStatus", "燃料状态"),
    ("stats.fuelRemaining", "燃料剩余"),
    ("stats.emberHeat", "余烬"),
    ("stats.intensity", "强度"),
    ("stats.total", "总量"),
    ("stats.bySource", "按工具"),
    ("stats.hourly", "今日节奏"),
    ("stats.breakdown", "用量去向"),
    ("stats.input", "输入"),
    ("stats.output", "输出"),
    ("stats.cacheRead", "缓存读"),
    ("stats.cacheWrite", "缓存写"),
    ("stats.other", "其他 / 估算"),
    ("stats.empty", "今天还没有用量记录。"),
    ("stats.tier", "火焰"),
    ("size.title", "火焰尺寸"),
    ("size.small", "小"),
    ("size.medium", "中"),
    ("size.large", "大"),
    ("size.hint", "会真正缩放桌面篝火，不只改窗口。"),
    ("sound.title", "壁炉音效"),
    ("sound.enabled", "开启"),
    ("sound.volume", "音量"),
    ("hover.settings.title", "悬停卡片"),
    ("debug.applyIntensity", "应用自定义强度"),
    ("menu.hideFlame", "隐藏火焰"),
    ("menu.showFlame", "显示火焰"),
    ("menu.toggleFlame", "显示 / 隐藏火焰"),
    ("menu.resetPosition", "重置到右下角"),
    ("menu.openConsole", "打开面板"),
    ("menu.checkUpdates", "检查更新…"),
    ("menu.pauseAnimation", "暂停动画"),
    ("menu.resumeAnimation", "恢复动画"),
    ("menu.togglePause", "暂停 / 恢复动画"),
    ("menu.quit", "退出 TokenBlaze"),
    ("settings.reduceMotion", "减少动态效果"),
    ("settings.showLiveRate", "显示实时 token 消耗"),
    (
        "settings.showLiveRate.hint",
        "仅为预估，依据近期本机用量波动，可能不完全准确。",
    ),
    (
        "settings.privacy",
        "读取本机 Codex、Claude Code、Cursor、Grok、Pi、Amp 和 OpenCode 用量，不上传任何数据。",
    ),
    ("tier.hush", "微火"),
    ("tier.glow", "小火"),
    ("tier.crackle", "中火"),
    ("tier.roar", "旺火"),
    ("tier.blaze", "烈火"),
    ("phase.unlit", "未点燃"),
    ("phase.flame", "明火"),
    ("phase.ember", "余烬"),
    ("phase.out", "已熄灭"),
    ("hover.today", "今日 tokens"),
    ("hover.rate", "token/s · 估"),
    ("hover.estimated", "估"),
    ("hover.updated", "更新于 %@"),
    ("update.title", "发现更新"),
    ("update.later", "稍后"),
    ("update.download", "下载"),
    ("update.checking", "正在检查更新…"),
    ("update.upToDate", "TokenBlaze 已是最新版本。"),
    ("update.checkAgain", "重新检查"),
    ("update.available", "发现新版本："),
    (
        "update.noWindowsPackage",
        "发现新版本，但没有适用于当前平台的安装包：",
    ),
    ("update.openReleases", "打开发布页面"),
    ("update.downloading", "正在下载"),
    ("update.ready", "已准备安装："),
    ("update.installRestart", "安装并重启"),
    ("update.failed", "更新失败"),
    ("update.retry", "重试"),
];

// ── Japanese ───────────────────────────────────────────────────

const JA: &[(&str, &str)] = &[
    ("app.name", "TokenBlaze"),
    (
        "app.tagline",
        "どうせトークンを燃やすなら、本物の火を灯そう。",
    ),
    ("console.title", "パネル"),
    ("console.productSubtitle", "ローカル AI 使用量の暖炉"),
    ("nav.overview", "概要"),
    ("nav.todayFire", "今日の火"),
    ("nav.manage", "管理"),
    ("nav.appearance", "炎の外観"),
    ("nav.preferences", "環境設定"),
    ("nav.burningNormally", "正常に燃焼中"),
    ("console.footnote", "ローカルの使用ログのみを読み取り、アップロードしません。"),
    ("console.version", "バージョン %@"),
    ("console.website", "ウェブサイト"),
    ("console.contact", "開発者に連絡"),
    ("debug.title", "デバッグ"),
    ("debug.hint", "炎の見た目を強制します。このウィンドウを閉じるとデバッグ項目は再び非表示になります。"),
    ("debug.live", "ライブ"),
    ("debug.inject", "トークンを追加"),
    ("debug.autoBurn", "自動燃焼"),
    ("debug.pause", "アニメーションを一時停止"),
    ("debug.intensity", "強度を指定"),
    ("language", "言語"),
    ("language.system", "システム"),
    ("language.english", "English"),
    ("language.chinese", "中文"),
    ("language.japanese", "日本語"),
    ("language.korean", "한국어"),
    ("sources.title", "データソース"),
    ("sources.rescan", "再スキャン"),
    ("sources.detail", "詳細"),
    ("sources.lastRead", "最終読み取り"),
    ("sources.notRead", "未読み取り"),
    ("sources.recent", "最近のイベント"),
    ("sources.noRecent", "最近のイベントはありません"),
    ("sources.estimated", "推定"),
    ("source.state.ok", "接続済み"),
    ("source.state.notFound", "未検出"),
    ("source.state.noPermission", "権限なし"),
    ("source.state.unsupported", "非対応"),
    ("source.state.readError", "読み取りエラー"),
    ("source.claude_code", "Claude Code"),
    ("source.codex", "Codex"),
    ("source.cursor", "Cursor"),
    ("source.grok", "Grok"),
    ("source.pi", "Pi"),
    ("source.amp", "Amp"),
    ("source.opencode", "OpenCode"),
    ("colors.title", "炎の色"),
    ("colors.reset", "リセット"),
    ("colors.mix.empty", "クラシックオレンジ — 使用量を待機中"),
    ("colors.mix.active", "現在の配合"),
    ("stats.title", "今日"),
    ("stats.todayTokens", "今日の TOKENS"),
    ("stats.currentState", "現在の状態"),
    ("stats.sourceMix", "ソース構成"),
    ("stats.fuelStatus", "燃料の状態"),
    ("stats.fuelRemaining", "燃料残量"),
    ("stats.emberHeat", "残り火"),
    ("stats.intensity", "強度"),
    ("stats.total", "合計"),
    ("stats.bySource", "ツール別"),
    ("stats.hourly", "一日の推移"),
    ("stats.breakdown", "トークンの内訳"),
    ("stats.input", "入力"),
    ("stats.output", "出力"),
    ("stats.cacheRead", "キャッシュ読み取り"),
    ("stats.cacheWrite", "キャッシュ書き込み"),
    ("stats.other", "その他 / 推定"),
    ("stats.empty", "今日はまだ使用記録がありません。"),
    ("stats.tier", "炎"),
    ("size.title", "炎のサイズ"),
    ("size.small", "S"),
    ("size.medium", "M"),
    ("size.large", "L"),
    ("size.hint", "ウィンドウだけでなく、デスクトップの炎全体を拡大縮小します。"),
    ("sound.title", "暖炉の音"),
    ("sound.enabled", "オン"),
    ("sound.volume", "音量"),
    ("hover.settings.title", "ホバーカード"),
    ("debug.applyIntensity", "カスタム強度を適用"),
    ("menu.hideFlame", "炎を隠す"),
    ("menu.showFlame", "炎を表示"),
    ("menu.toggleFlame", "炎を表示 / 非表示"),
    ("menu.resetPosition", "位置をリセット"),
    ("menu.openConsole", "パネルを開く"),
    ("menu.checkUpdates", "アップデートを確認…"),
    ("menu.pauseAnimation", "アニメ一時停止"),
    ("menu.resumeAnimation", "アニメ再開"),
    ("menu.togglePause", "アニメーションを一時停止 / 再開"),
    ("menu.quit", "TokenBlaze を終了"),
    ("settings.reduceMotion", "動きを減らす"),
    ("settings.showLiveRate", "リアルタイムのトークン速度を表示"),
    ("settings.showLiveRate.hint", "最近のローカル使用量に基づく推定値で、正確でない場合があります。"),
    ("settings.privacy", "Codex、Claude Code、Cursor、Grok、Pi、Amp、OpenCode のローカル使用量を読み取ります。アップロードはしません。"),
    ("hover.today", "今日のトークン"),
    ("hover.rate", "token/s · 推定"),
    ("hover.estimated", "推定"),
    ("hover.updated", "%@ に更新"),
    ("update.title", "アップデートがあります"),
    ("update.later", "後で"),
    ("update.download", "ダウンロード"),
    ("update.checking", "アップデートを確認中…"),
    ("update.upToDate", "TokenBlaze は最新です。"),
    ("update.checkAgain", "もう一度確認"),
    ("update.available", "新しいバージョン："),
    (
        "update.noWindowsPackage",
        "新しいバージョンがありますが、このプラットフォーム向けのパッケージはありません：",
    ),
    ("update.openReleases", "リリースを開く"),
    ("update.downloading", "ダウンロード中"),
    ("update.ready", "インストール準備完了："),
    ("update.installRestart", "インストールして再起動"),
    ("update.failed", "アップデートに失敗しました"),
    ("update.retry", "再試行"),
    ("tier.hush", "静火"),
    ("tier.glow", "ほのか"),
    ("tier.crackle", "ぱちぱち"),
    ("tier.roar", "ごうごう"),
    ("tier.blaze", "烈火"),
    ("phase.unlit", "未点火"),
    ("phase.flame", "炎"),
    ("phase.ember", "残り火"),
    ("phase.out", "消火"),
];

// ── Korean ─────────────────────────────────────────────────────

const KO: &[(&str, &str)] = &[
    ("app.name", "TokenBlaze"),
    (
        "app.tagline",
        "어차피 토큰을 태울 거라면, 진짜 불을 피우자.",
    ),
    ("console.title", "패널"),
    ("console.productSubtitle", "로컬 AI 사용량 벽난로"),
    ("nav.overview", "개요"),
    ("nav.todayFire", "오늘의 불꽃"),
    ("nav.manage", "관리"),
    ("nav.appearance", "불꽃 모양"),
    ("nav.preferences", "환경설정"),
    ("nav.burningNormally", "정상 연소 중"),
    ("console.footnote", "로컬 사용량 로그만 읽으며 업로드하지 않습니다."),
    ("console.version", "버전 %@"),
    ("console.website", "웹사이트"),
    ("console.contact", "개발자에게 문의"),
    ("debug.title", "디버그"),
    ("debug.hint", "불꽃 모양을 강제로 미리 봅니다. 이 창을 닫으면 디버그 항목이 다시 숨겨집니다."),
    ("debug.live", "실시간"),
    ("debug.inject", "토큰 주입"),
    ("debug.autoBurn", "자동 연소"),
    ("debug.pause", "애니메이션 일시정지"),
    ("debug.intensity", "강도 지정"),
    ("language", "언어"),
    ("language.system", "시스템"),
    ("language.english", "English"),
    ("language.chinese", "中文"),
    ("language.japanese", "日本語"),
    ("language.korean", "한국어"),
    ("sources.title", "데이터 소스"),
    ("sources.rescan", "다시 스캔"),
    ("sources.detail", "세부 정보"),
    ("sources.lastRead", "마지막 읽기"),
    ("sources.notRead", "아직 읽지 않음"),
    ("sources.recent", "최근 이벤트"),
    ("sources.noRecent", "최근 이벤트 없음"),
    ("sources.estimated", "추정"),
    ("source.state.ok", "연결됨"),
    ("source.state.notFound", "없음"),
    ("source.state.noPermission", "권한 없음"),
    ("source.state.unsupported", "지원 안 함"),
    ("source.state.readError", "읽기 오류"),
    ("source.claude_code", "Claude Code"),
    ("source.codex", "Codex"),
    ("source.cursor", "Cursor"),
    ("source.grok", "Grok"),
    ("source.pi", "Pi"),
    ("source.amp", "Amp"),
    ("source.opencode", "OpenCode"),
    ("colors.title", "불꽃 색"),
    ("colors.reset", "초기화"),
    ("colors.mix.empty", "기본 주황색 — 사용량 대기 중"),
    ("colors.mix.active", "실시간 혼합"),
    ("stats.title", "오늘"),
    ("stats.todayTokens", "오늘의 TOKENS"),
    ("stats.currentState", "현재 상태"),
    ("stats.sourceMix", "소스 구성"),
    ("stats.fuelStatus", "연료 상태"),
    ("stats.fuelRemaining", "남은 연료"),
    ("stats.emberHeat", "잔불"),
    ("stats.intensity", "강도"),
    ("stats.total", "합계"),
    ("stats.bySource", "도구별"),
    ("stats.hourly", "하루 흐름"),
    ("stats.breakdown", "토큰 사용 내역"),
    ("stats.input", "입력"),
    ("stats.output", "출력"),
    ("stats.cacheRead", "캐시 읽기"),
    ("stats.cacheWrite", "캐시 쓰기"),
    ("stats.other", "기타 / 추정"),
    ("stats.empty", "오늘 기록된 사용량이 없습니다."),
    ("stats.tier", "불꽃"),
    ("size.title", "불꽃 크기"),
    ("size.small", "S"),
    ("size.medium", "M"),
    ("size.large", "L"),
    ("size.hint", "창뿐 아니라 데스크톱 불꽃 전체의 크기를 조절합니다."),
    ("sound.title", "벽난로 소리"),
    ("sound.enabled", "켜기"),
    ("sound.volume", "음량"),
    ("hover.settings.title", "호버 카드"),
    ("debug.applyIntensity", "사용자 지정 강도 적용"),
    ("menu.hideFlame", "불꽃 숨기기"),
    ("menu.showFlame", "불꽃 보이기"),
    ("menu.toggleFlame", "불꽃 보이기 / 숨기기"),
    ("menu.resetPosition", "위치 초기화"),
    ("menu.openConsole", "패널 열기"),
    ("menu.checkUpdates", "업데이트 확인…"),
    ("menu.pauseAnimation", "애니메이션 일시정지"),
    ("menu.resumeAnimation", "애니메이션 재개"),
    ("menu.togglePause", "애니메이션 일시정지 / 재개"),
    ("menu.quit", "TokenBlaze 종료"),
    ("settings.reduceMotion", "움직임 줄이기"),
    ("settings.showLiveRate", "실시간 토큰 속도 표시"),
    ("settings.showLiveRate.hint", "최근 로컬 사용량을 바탕으로 한 추정치이며 정확하지 않을 수 있습니다."),
    ("settings.privacy", "로컬 Codex, Claude Code, Cursor, Grok, Pi, Amp 및 OpenCode 사용량을 읽습니다. 업로드하지 않습니다."),
    ("hover.today", "오늘 토큰"),
    ("hover.rate", "token/s · 추정"),
    ("hover.estimated", "추정"),
    ("hover.updated", "%@에 업데이트"),
    ("update.title", "업데이트 있음"),
    ("update.later", "나중에"),
    ("update.download", "다운로드"),
    ("update.checking", "업데이트 확인 중…"),
    ("update.upToDate", "TokenBlaze가 최신 버전입니다."),
    ("update.checkAgain", "다시 확인"),
    ("update.available", "새 버전："),
    (
        "update.noWindowsPackage",
        "새 버전이 있지만 현재 플랫폼용 패키지가 없습니다：",
    ),
    ("update.openReleases", "릴리스 열기"),
    ("update.downloading", "다운로드 중"),
    ("update.ready", "설치 준비 완료："),
    ("update.installRestart", "설치 후 다시 시작"),
    ("update.failed", "업데이트 실패"),
    ("update.retry", "다시 시도"),
    ("tier.hush", "고요"),
    ("tier.glow", "은은"),
    ("tier.crackle", "타닥"),
    ("tier.roar", "활활"),
    ("tier.blaze", "맹렬"),
    ("phase.unlit", "미점화"),
    ("phase.flame", "불꽃"),
    ("phase.ember", "잔불"),
    ("phase.out", "꺼짐"),
];

/// Simple lookup from a small key-value slice.
fn lookup(table: &[(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, value)| *value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn finds_keys_even_when_table_order_is_not_sorted() {
        assert_eq!(t("menu.quit", AppLanguage::English), "Quit TokenBlaze");
        assert_eq!(t("phase.out", AppLanguage::Chinese), "已熄灭");
    }

    #[test]
    fn maps_supported_locale_names() {
        assert_eq!(language_from_locale("zh-CN"), Some(AppLanguage::Chinese));
        assert_eq!(
            language_from_locale("JA_jp.UTF-8"),
            Some(AppLanguage::Japanese)
        );
        assert_eq!(language_from_locale("ko-KR"), Some(AppLanguage::Korean));
        assert_eq!(language_from_locale("en-US"), Some(AppLanguage::English));
        assert_eq!(language_from_locale("fr-FR"), None);
    }

    #[test]
    fn every_translation_table_matches_english_keys() {
        let english: HashSet<_> = EN.iter().map(|(key, _)| *key).collect();
        assert_eq!(english.len(), EN.len(), "English table has duplicate keys");
        for (name, table) in [("ZH", ZH), ("JA", JA), ("KO", KO)] {
            let translated: HashSet<_> = table.iter().map(|(key, _)| *key).collect();
            assert_eq!(translated.len(), table.len(), "{name} has duplicate keys");
            assert_eq!(translated, english, "{name} does not match English keys");
        }
    }
}
