const $ = (id) => document.getElementById(id);
const sourceNames = ['Claude Code', 'Codex', 'Cursor', 'Grok', 'Pi', 'Amp', 'OpenCode'];
const builtInColors = ['#e8782e', '#47b86b', '#528feb', '#b86bf2', '#f29e38', '#33c7c7', '#ec4899'];
let sourceColors = [...builtInColors];
let snapshot = null;
let overviewFlame = null;

const translationRows = [
  ['window.title', 'TokenBlaze Console', 'TokenBlaze 控制台', 'TokenBlaze コンソール', 'TokenBlaze 콘솔'],
  ['brand.subtitle', 'activity console', '活动控制台', 'アクティビティコンソール', '활동 콘솔'],
  ['status.local', 'Local only', '仅限本机', 'ローカルのみ', '로컬 전용'],
  ['nav.overview', 'Overview', '概览', '概要', '개요'],
  ['nav.sources', 'Sources', '数据源', 'データソース', '데이터 소스'],
  ['nav.appearance', 'Appearance', '火焰外观', '炎の外観', '불꽃 모양'],
  ['nav.settings', 'Settings', '偏好设置', '環境設定', '환경설정'],
  ['header.liveActivity', 'LIVE ACTIVITY', '实时活动', 'ライブ活動', '실시간 활동'],
  ['header.console', 'TOKENBLAZE CONSOLE', 'TOKENBLAZE 控制台', 'TOKENBLAZE コンソール', 'TOKENBLAZE 콘솔'],
  ['header.live', 'LIVE', '实时', 'ライブ', '실시간'],
  ['overview.currentFlame', 'CURRENT FLAME', '当前火焰', '現在の炎', '현재 불꽃'],
  ['overview.intro', 'Live usage across your connected sources, shaped into one clear view.', '查看已连接数据源的实时用量概况。', '接続済みソースの使用状況をリアルタイムで確認できます。', '연결된 소스의 실시간 사용량을 한눈에 확인합니다.'],
  ['overview.fuel', 'fuel', '燃料', '燃料', '연료'],
  ['overview.intensity', 'intensity', '强度', '強度', '강도'],
  ['overview.emberHeat', 'ember heat', '余烬热度', '残り火', '잔불 열기'],
  ['overview.todayUsage', "TODAY'S USAGE", '今日用量', '今日の使用量', '오늘 사용량'],
  ['overview.tokensObserved', 'tokens observed', '已记录 token', '観測トークン', '관측된 토큰'],
  ['overview.acrossSources', 'across 7 sources', '来自 7 个数据源', '7 個のソース', '7개 소스 전체'],
  ['overview.sourceMix', 'SOURCE MIX', '来源构成', 'ソース構成', '소스 구성'],
  ['overview.activityRhythm', 'ACTIVITY RHYTHM', '活动节奏', 'アクティビティ推移', '활동 흐름'],
  ['overview.last24Hours', 'Last 24 hours', '最近 24 小时', '過去 24 時間', '최근 24시간'],
  ['overview.todayByHour', 'Today by hour', '今日逐小时', '今日の時間別', '오늘 시간별'],
  ['overview.sevenDays', 'Last 7 days', '最近 7 天', '過去 7 日間', '최근 7일'],
  ['overview.exportPng', "Export today's card as PNG", '导出今日用量卡片 PNG', '今日の利用カードを PNG で保存', '오늘 사용량 카드를 PNG로 내보내기'],
  ['overview.tokenBreakdown', 'TOKEN BREAKDOWN', 'TOKEN 明细', 'トークン内訳', '토큰 내역'],
  ['overview.whereUsageWent', 'Where usage went', '用量去向', '使用量の内訳', '사용량 구성'],
  ['stats.input', 'Input', '输入', '入力', '입력'],
  ['stats.output', 'Output', '输出', '出力', '출력'],
  ['stats.cacheRead', 'Cache read', '缓存读取', 'キャッシュ読み取り', '캐시 읽기'],
  ['stats.cacheWrite', 'Cache write', '缓存写入', 'キャッシュ書き込み', '캐시 쓰기'],
  ['sources.eyebrow', 'OBSERVABILITY', '可观测性', '可観測性', '관측성'],
  ['sources.title', 'Source connections', '数据源连接', 'データソース接続', '데이터 소스 연결'],
  ['sources.intro', 'Every source stays local. TokenBlaze reads usage events and turns them into one calm signal.', '所有数据都留在本机。TokenBlaze 读取用量事件，并将它们汇总为平稳的活动信号。', 'すべてのデータは端末内に留まります。TokenBlaze は使用イベントを読み取り、ひとつの穏やかな信号にまとめます。', '모든 데이터는 기기에만 남습니다. TokenBlaze는 사용 이벤트를 읽어 하나의 차분한 신호로 만듭니다.'],
  ['sources.source', 'SOURCE', '数据源', 'ソース', '소스'],
  ['sources.connected', 'CONNECTED', '已连接', '接続済み', '연결됨'],
  ['sources.lastRead', 'Last read: {time}', '最近读取：{time}', '最終読み取り: {time}', '마지막 읽기: {time}'],
  ['sources.notReadYet', 'No successful read yet', '尚未成功读取', 'まだ読み取りに成功していません', '아직 성공적으로 읽지 못함'],
  ['sources.estimated', 'Estimated: {tokens} tokens', '估算：{tokens} tokens', '推定: {tokens} tokens', '추정: {tokens} tokens'],
  ['sources.notFound', 'NOT FOUND', '未发现', '未検出', '찾을 수 없음'],
  ['sources.noPermission', 'NO PERMISSION', '无权限', '権限なし', '권한 없음'],
  ['sources.unsupported', 'UNSUPPORTED', '不支持', '非対応', '지원 안 함'],
  ['sources.readError', 'READ ERROR', '读取错误', '読み取りエラー', '읽기 오류'],
  ['sources.noDetail', 'No status detail', '暂无状态详情', '状態の詳細はありません', '상태 세부 정보 없음'],
  ['sources.tokens', 'tokens', 'token', 'トークン', '토큰'],
  ['sources.noHome', 'No home directory', '未找到用户主目录', 'ホームディレクトリがありません', '홈 디렉토리를 찾을 수 없음'],
  ['sources.piNotFound', 'Pi logs not found', '未找到 Pi 日志', 'Pi のログが見つかりません', 'Pi 로그를 찾을 수 없음'],
  ['sources.openCodeMemory', 'OpenCode uses an in-memory database', 'OpenCode 正在使用内存数据库', 'OpenCode はメモリ内データベースを使用しています', 'OpenCode가 메모리 내 데이터베이스를 사용 중임'],
  ['sources.openCodeNotFound', 'OpenCode database not found', '未找到 OpenCode 数据库', 'OpenCode データベースが見つかりません', 'OpenCode 데이터베이스를 찾을 수 없음'],
  ['sources.openCodeUnreadable', 'Cannot read OpenCode database', '无法读取 OpenCode 数据库', 'OpenCode データベースを読み取れません', 'OpenCode 데이터베이스를 읽을 수 없음'],
  ['sources.openCodeUnsupported', 'Unsupported OpenCode database schema', '不支持的 OpenCode 数据库结构', '未対応の OpenCode データベース構造です', '지원하지 않는 OpenCode 데이터베이스 스키마'],
  ['sources.cursorMissing', 'Missing Cursor state.vscdb', '未找到 Cursor state.vscdb', 'Cursor の state.vscdb がありません', 'Cursor state.vscdb를 찾을 수 없음'],
  ['sources.cursorReady', 'Dashboard API ready', '仪表板 API 已就绪', 'ダッシュボード API は準備完了です', '대시보드 API 준비됨'],
  ['sources.cursorLocalNoToken', 'Local estimate (no Cursor token)', '本地估算（无 Cursor token）', 'ローカル推定（Cursor トークンなし）', '로컬 추정(Cursor 토큰 없음)'],
  ['sources.cursorNotFound', 'Cursor not found', '未找到 Cursor', 'Cursor が見つかりません', 'Cursor를 찾을 수 없음'],
  ['sources.cursorDashboard', 'Dashboard API', '仪表板 API', 'ダッシュボード API', '대시보드 API'],
  ['sources.cursorUnavailable', 'Dashboard API (temporarily unavailable)', '仪表板 API（暂时不可用）', 'ダッシュボード API（一時的に利用不可）', '대시보드 API(일시적으로 사용 불가)'],
  ['sources.cursorRateLimited', 'Dashboard API (rate limited)', '仪表板 API（已限流）', 'ダッシュボード API（レート制限中）', '대시보드 API(요청 제한됨)'],
  ['sources.cursorLocal', 'Local estimate', '本地估算', 'ローカル推定', '로컬 추정'],
  ['sources.ampNotFound', 'Amp logs not found', '未找到 Amp 日志', 'Amp のログが見つかりません', 'Amp 로그를 찾을 수 없음'],
  ['sources.grokNotFound', 'Grok logs not found', '未找到 Grok 日志', 'Grok のログが見つかりません', 'Grok 로그를 찾을 수 없음'],
  ['appearance.eyebrow', 'VISUAL SYSTEM', '视觉系统', 'ビジュアルシステム', '비주얼 시스템'],
  ['appearance.title', 'Appearance', '火焰外观', '炎の外観', '불꽃 모양'],
  ['appearance.intro', 'Shape the native flame while keeping the same quiet desktop presence.', '调整原生火焰，同时保持安静克制的桌面体验。', '静かなデスクトップの存在感を保ちながら、ネイティブの炎を調整します。', '차분한 데스크톱 분위기를 유지하면서 네이티브 불꽃을 조절합니다.'],
  ['appearance.reduceMotion', 'Reduce motion', '减少动态效果', '動きを減らす', '움직임 줄이기'],
  ['appearance.reduceMotionHint', 'Keep the flame calm and conserve energy.', '让火焰更加平缓并节省资源。', '炎を穏やかにして消費電力を抑えます。', '불꽃을 차분하게 유지하고 에너지를 절약합니다.'],
  ['appearance.showRate', 'Show live rate', '显示实时速率', 'ライブ速度を表示', '실시간 속도 표시'],
  ['appearance.showRateHint', 'Display the current activity rhythm in the overview.', '在概览中显示当前活动速率。', '概要に現在のアクティビティ速度を表示します。', '개요에 현재 활동 속도를 표시합니다.'],
  ['appearance.pause', 'Pause animation', '暂停动画', 'アニメーションを一時停止', '애니메이션 일시정지'],
  ['appearance.pauseHint', 'Freeze movement without stopping usage monitoring.', '冻结火焰动画，但不中断用量监测。', '使用量の監視を止めずに動きだけを停止します。', '사용량 모니터링을 유지한 채 움직임만 멈춥니다.'],
  ['appearance.flameSize', 'Flame size', '火焰尺寸', '炎のサイズ', '불꽃 크기'],
  ['appearance.flameSizeHint', 'Resize the native desktop flame.', '调整原生桌面火焰的大小。', 'デスクトップのネイティブ炎を拡大縮小します。', '네이티브 데스크톱 불꽃의 크기를 조절합니다.'],
  ['appearance.sourceColors', 'SOURCE COLORS', '数据源颜色', 'ソースの色', '소스 색상'],
  ['appearance.reset', 'Reset', '恢复默认', 'リセット', '초기화'],
  ['appearance.preview', 'FLAME PREVIEW', '火焰预览', '炎のプレビュー', '불꽃 미리보기'],
  ['appearance.previewButton', 'Preview', '预览', 'プレビュー', '미리보기'],
  ['appearance.returnLive', 'Return to live', '返回实时状态', 'ライブに戻る', '실시간으로 돌아가기'],
  ['appearance.color', 'color', '颜色', '色', '색상'],
  ['settings.eyebrow', 'PREFERENCES', '偏好设置', '環境設定', '환경설정'],
  ['settings.title', 'Settings', '设置', '設定', '설정'],
  ['settings.intro', 'Small controls for a quiet desktop companion.', '为这位安静的桌面伙伴提供简洁控制。', '静かなデスクトップコンパニオンのためのシンプルな設定です。', '조용한 데스크톱 동반자를 위한 간단한 설정입니다.'],
  ['settings.sound', 'Sound bed', '壁炉音效', '暖炉の音', '벽난로 소리'],
  ['settings.soundHint', 'Ambient crackle follows the same local usage signal.', '环境噼啪声会跟随同一份本机用量信号。', '環境音は同じローカル使用量信号に連動します。', '주변 타닥거림이 동일한 로컬 사용량 신호를 따릅니다.'],
  ['error.action', 'This action failed. Check the source state and try again.', '操作失败。请检查数据源状态后重试。', '操作に失敗しました。ソースの状態を確認して再試行してください。', '작업에 실패했습니다. 소스 상태를 확인하고 다시 시도하세요.'],
  ['settings.volume', 'Volume', '音量', '音量', '음량'],
  ['settings.panel', 'Flame panel', '火焰面板', '炎パネル', '불꽃 패널'],
  ['settings.panelHint', 'Show or hide the native desktop flame without closing TokenBlaze.', '显示或隐藏原生桌面火焰，而不退出 TokenBlaze。', 'TokenBlaze を終了せずにデスクトップの炎を表示・非表示にします。', 'TokenBlaze를 종료하지 않고 데스크톱 불꽃을 표시하거나 숨깁니다.'],
  ['settings.language', 'Language', '语言', '言語', '언어'],
  ['settings.languageHint', 'Apply to the console, tray, and flame panel.', '应用到控制台、托盘菜单和火焰面板。', 'コンソール、トレイ、炎パネルに適用します。', '콘솔, 트레이, 불꽃 패널에 적용합니다.'],
  ['settings.pollFrequency', 'Token polling frequency', 'Token 获取频率', 'トークン取得間隔', '토큰 가져오기 간격'],
  ['settings.pollFrequencyHint', 'Check local sources for new usage at this interval.', '每隔此时间检查本机数据源中的新增用量。', 'この間隔でローカルソースの新しい使用量を確認します。', '이 간격으로 로컬 소스의 새 사용량을 확인합니다.'],
  ['settings.seconds1', '1 second', '1 秒', '1 秒', '1초'],
  ['settings.seconds2', '2 seconds', '2 秒', '2 秒', '2초'],
  ['settings.seconds5', '5 seconds', '5 秒', '5 秒', '5초'],
  ['settings.seconds10', '10 seconds', '10 秒', '10 秒', '10초'],
  ['settings.rescan', 'Rescan sources', '重新扫描数据源', 'データソースを再スキャン', '데이터 소스 다시 스캔'],
  ['settings.rescanWorking', 'Scanning sources…', '正在扫描数据源…', 'データソースをスキャン中…', '데이터 소스 스캔 중…'],
  ['settings.rescanFailed', 'Could not start the source scan.', '无法启动数据源扫描。', 'データソースのスキャンを開始できませんでした。', '데이터 소스 스캔을 시작하지 못했습니다.'],
  ['settings.sourcePath', 'Local session folder', '本机会话目录', 'ローカルセッションフォルダー', '로컬 세션 폴더'],
  ['settings.sourcePathHint', 'Paste a folder path for Claude Code, Codex, Grok, Pi, or Amp. Leave empty to use automatic detection.', '为 Claude Code、Codex、Grok、Pi 或 Amp 粘贴日志目录；留空则自动检测。', 'Claude Code、Codex、Grok、Pi、Amp のログフォルダーを入力します。空欄なら自動検出します。', 'Claude Code, Codex, Grok, Pi 또는 Amp의 로그 폴더 경로를 붙여넣으세요. 비우면 자동으로 찾습니다.'],
  ['settings.savePath', 'Save folder', '保存目录', '保存', '폴더 저장'],
  ['settings.clearPath', 'Automatic', '恢复自动检测', '自動検出', '자동 검색'],
  ['settings.pathSaved', 'Folder saved. Scanning now.', '目录已保存，正在扫描。', 'フォルダーを保存しました。スキャン中です。', '폴더를 저장했습니다. 스캔 중입니다.'],
  ['settings.debug', 'DEBUG SIGNAL', '调试信号', 'デバッグ信号', '디버그 신호'],
  ['settings.inject', 'Inject tokens', '注入 token', 'トークンを追加', '토큰 주입'],
  ['settings.updates', 'Updates', '更新', 'アップデート', '업데이트'],
  ['settings.updateReady', 'Ready to check.', '可以检查更新。', '確認できます。', '업데이트를 확인할 수 있습니다.'],
  ['settings.check', 'Check', '检查', '確認', '확인'],
  ['settings.download', 'Download', '下载', 'ダウンロード', '다운로드'],
  ['settings.install', 'Install & restart', '安装并重启', 'インストールして再起動', '설치 후 다시 시작'],
  ['settings.dismiss', 'Dismiss', '忽略', '閉じる', '닫기'],
  ['settings.privacy', 'Private by design', '隐私优先设计', 'プライバシー重視', '개인정보 보호 중심'],
  ['settings.privacyHint', "No usage data leaves this device. The console shares the native app's local store.", '用量数据不会离开此设备。控制台与原生应用共用本地存储。', '使用量データが端末外へ送信されることはありません。コンソールはネイティブアプリのローカルストアを共有します。', '사용량 데이터는 이 기기를 벗어나지 않습니다. 콘솔은 네이티브 앱의 로컬 저장소를 공유합니다.'],
  ['language.system', 'System', '跟随系统', 'システム', '시스템'],
  ['language.english', 'English', 'English', 'English', 'English'],
  ['language.chinese', '中文', '中文', '中文', '中文'],
  ['language.japanese', '日本語', '日本語', '日本語', '日本語'],
  ['language.korean', '한국어', '한국어', '한국어', '한국어'],
  ['size.small', 'Small', '小', '小', '소'], ['size.medium', 'Medium', '中', '中', '중'], ['size.large', 'Large', '大', '大', '대'],
  ['tier.out', 'Out', '熄灭', '消火', '꺼짐'], ['tier.ember', 'Ember', '余烬', '残り火', '잔불'],
  ['tier.hush', 'Hush', '微火', '静火', '고요'], ['tier.glow', 'Glow', '小火', 'ほのか', '은은'],
  ['tier.crackle', 'Crackle', '中火', 'ぱちぱち', '타닥'], ['tier.roar', 'Roar', '旺火', 'ごうごう', '활활'], ['tier.blaze', 'Blaze', '烈火', '烈火', '맹렬'],
  ['tier.outLabel', 'Fire out', '火焰已熄灭', '消火中', '불꽃 꺼짐'], ['tier.emberLabel', 'Warm embers', '温热余烬', '暖かな残り火', '따뜻한 잔불'],
  ['tier.hushLabel', 'Quiet glow', '微弱火光', '静かな灯り', '조용한 불빛'], ['tier.glowLabel', 'Warm glow', '温暖火光', '暖かな灯り', '따뜻한 불빛'],
  ['tier.crackleLabel', 'Soft crackle', '轻柔噼啪', '穏やかな炎', '부드러운 타닥임'], ['tier.roarLabel', 'Bright roar', '明亮旺火', '明るい炎', '밝은 불꽃'],
  ['tier.blazeLabel', 'Full blaze', '熊熊烈火', '燃え盛る炎', '거센 불꽃'],
  ['phase.unlit', 'UNLIT', '未点燃', '未点火', '미점화'], ['phase.flame', 'FLAME', '明火', '炎', '불꽃'],
  ['phase.ember', 'EMBER', '余烬', '残り火', '잔불'], ['phase.out', 'OUT', '已熄灭', '消火', '꺼짐'],
  ['dynamic.active', 'active', '个活跃', '件有効', '개 활성'], ['dynamic.rate', 'token/s · est.', 'token/s · 估', 'token/s · 推定', 'token/s · 추정'],
  ['dynamic.connected', 'connected', '个已连接', '件接続済み', '개 연결됨'],
  ['sources.firstRun', 'If a source is missing, install or open that tool, check its local usage logs and permissions, then rescan. TokenBlaze reads local files only.', '如果未发现数据源，请先安装或打开对应工具，检查本地用量日志和文件权限，再重新扫描。TokenBlaze 只读取本地文件。', 'ソースが見つからない場合は、対象ツールをインストールまたは起動し、ローカルログとアクセス権を確認して再スキャンしてください。TokenBlaze はローカルファイルのみを読み取ります。', '소스를 찾을 수 없으면 해당 도구를 설치하거나 열고, 로컬 사용 로그와 권한을 확인한 뒤 다시 스캔하세요. TokenBlaze는 로컬 파일만 읽습니다.'],
  ['dynamic.peak', 'peak {hour}:00', '峰值 {hour}:00', 'ピーク {hour}:00', '최고 {hour}:00'],
  ['update.checking', 'Checking for updates…', '正在检查更新…', 'アップデートを確認中…', '업데이트 확인 중…'],
  ['update.upToDate', 'TokenBlaze is up to date.', 'TokenBlaze 已是最新版本。', 'TokenBlaze は最新です。', 'TokenBlaze가 최신 버전입니다.'],
  ['update.available', 'Version {version} is available.', '发现新版本 {version}。', 'バージョン {version} が利用できます。', '버전 {version}을 사용할 수 있습니다.'],
  ['update.incompatible', 'Version {version} has no package for this platform.', '版本 {version} 没有适用于当前平台的安装包。', 'バージョン {version} にはこのプラットフォーム向けパッケージがありません。', '버전 {version}에 현재 플랫폼용 패키지가 없습니다.'],
  ['update.downloading', 'Downloading {version}…', '正在下载 {version}…', '{version} をダウンロード中…', '{version} 다운로드 중…'],
  ['update.ready', 'Version {version} is ready to install.', '版本 {version} 已准备安装。', 'バージョン {version} をインストールできます。', '버전 {version}을 설치할 수 있습니다.'],
  ['update.failed', 'Update failed.', '更新失败。', 'アップデートに失敗しました。', '업데이트 실패.'],
];
const languageNames = ['English', 'Chinese', 'Japanese', 'Korean'];
const copy = Object.fromEntries(languageNames.map((language, languageIndex) => [language, Object.fromEntries(translationRows.map((row) => [row[0], row[languageIndex + 1]]))]));
let selectedLanguage = 'English';

async function invoke(command, args = {}) {
  const api = window.__TAURI__?.core?.invoke;
  if (api) return api(command, args);
  if (command === 'dashboard_snapshot') return mockSnapshot();
  return undefined;
}

function mockSnapshot() {
  const t = Date.now() / 1000;
  const hourly = Array.from({ length: 24 }, (_, hour) => ({ hour, tokens: Math.round(Math.max(0, Math.sin((hour - 4) / 3) * 1600 + Math.random() * 420)) }));
  return {
    today_tokens: 18240,
    today_by_source: [8200, 4200, 2500, 1600, 900, 540, 300],
    hourly,
    last_seven_days: Array.from({ length: 7 }, (_, i) => ({ date: new Date(Date.now() - (6 - i) * 86400000).toISOString().slice(0, 10), tokens: Math.round(4000 + Math.random() * 14000) })),
    breakdown: { input: 10100, output: 5340, cache_read: 2100, cache_write: 700 },
    fire: { intensity: .64 + Math.sin(t) * .08, fuel: .72, ember_heat: .45, spark_burst: .2, phase: 'flame', tier: 'crackle', color_mix: [.5, .25, .1, .05, .04, .03, .03], tokens_per_second: 24.8, animation_paused: false, previewing: false },
    sources: sourceNames.map((name, i) => ({ id: name.toLowerCase(), name, tokens: [8200, 4200, 2500, 1600, 900, 540, 300][i], state: i < 5 ? 'ok' : 'notFound', detail: i < 5 ? 'Connected' : 'Not detected' })),
    debug_tools_enabled: false,
    config: { soundEnabled: false, soundVolume: .48, reduceMotion: false, showLiveRate: true, tokenPollIntervalSeconds: 2, panelVisible: true, language: 'English', flameSize: 'Medium', sourceColors: builtInColors.map(hexToRgb) },
    update: { state: 'idle', version: null, progress: null, message: null },
  };
}

function formatTokens(value) {
  return new Intl.NumberFormat(document.documentElement.lang || undefined, { notation: value > 999999 ? 'compact' : 'standard', maximumFractionDigits: 1 }).format(value || 0);
}

function configValue(config, camel, snake) { return config?.[camel] ?? config?.[snake]; }
function enumTitle(value) { const text = String(value || ''); return text.charAt(0).toUpperCase() + text.slice(1); }
function rgbToHex(rgb) { return `#${rgb.map((value) => Number(value).toString(16).padStart(2, '0')).join('')}`; }
function hexToRgb(hex) { return [1, 3, 5].map((start) => parseInt(hex.slice(start, start + 2), 16)); }

function effectiveLanguage(language) {
  if (language !== 'System') return copy[language] ? language : 'English';
  const locale = navigator.language.toLowerCase();
  if (locale.startsWith('zh')) return 'Chinese';
  if (locale.startsWith('ja')) return 'Japanese';
  if (locale.startsWith('ko')) return 'Korean';
  return 'English';
}

function tr(key, variables = {}) {
  const template = copy[selectedLanguage]?.[key] ?? copy.English[key] ?? key;
  return Object.entries(variables).reduce((text, [name, value]) => text.replaceAll(`{${name}}`, value), template);
}

const pollFrequencyRow = document.createElement('div');
pollFrequencyRow.className = 'setting-row';
pollFrequencyRow.innerHTML = '<div><strong data-i18n="settings.pollFrequency"></strong><small data-i18n="settings.pollFrequencyHint"></small></div><select id="token-poll-interval"><option value="1" data-i18n="settings.seconds1"></option><option value="2" data-i18n="settings.seconds2"></option><option value="5" data-i18n="settings.seconds5"></option><option value="10" data-i18n="settings.seconds10"></option></select>';
document.querySelector('#view-settings .settings-card').prepend(pollFrequencyRow);
const scrollbarStyle = document.createElement('style');
scrollbarStyle.textContent = '*{scrollbar-width:thin;scrollbar-color:rgba(197,174,160,.05) transparent}*:hover{scrollbar-color:rgba(197,174,160,.2) transparent}::-webkit-scrollbar{width:6px;height:6px}::-webkit-scrollbar-track{background:transparent}::-webkit-scrollbar-thumb{background:rgba(197,174,160,.045);border:2px solid transparent;background-clip:padding-box;border-radius:99px;transition:background-color .18s ease}*:hover::-webkit-scrollbar-thumb{background-color:rgba(197,174,160,.2)}::-webkit-scrollbar-thumb:hover{background-color:rgba(232,166,108,.42)}';
document.head.append(scrollbarStyle);

function applyLanguage(language) {
  selectedLanguage = effectiveLanguage(language);
  document.documentElement.lang = { Chinese: 'zh-CN', Japanese: 'ja', Korean: 'ko' }[selectedLanguage] || 'en';
  document.title = tr('window.title');
  document.querySelectorAll('[data-i18n]').forEach((element) => { element.textContent = tr(element.dataset.i18n); });
  document.querySelectorAll('[data-i18n-aria-label]').forEach((element) => { element.setAttribute('aria-label', tr(element.dataset.i18nAriaLabel)); });
}

function updateSnapshot(next) {
  snapshot = next;
  const config = next.config || {};
  overviewFlame?.setState({ fire: next.fire, reduceMotion: Boolean(configValue(config, 'reduceMotion', 'reduce_motion')) });
  const language = config.language || 'English';
  applyLanguage(language);
  const rescanButton = $('rescan');
  rescanButton.disabled = Boolean(next.is_rescanning);
  rescanButton.setAttribute('aria-busy', String(Boolean(next.is_rescanning)));
  rescanButton.querySelector('[data-i18n]').textContent = tr(next.is_rescanning ? 'settings.rescanWorking' : 'settings.rescan');
  const phase = enumTitle(next.fire.phase);
  const tier = enumTitle(next.fire.tier);
  const colors = configValue(config, 'sourceColors', 'source_colors');
  if (Array.isArray(colors) && colors.length === 7) sourceColors = colors.map(rgbToHex);

  $('token-value').textContent = formatTokens(next.today_tokens);
  $('fuel-value').textContent = `${Math.round((next.fire.fuel || 0) * 100)}%`;
  $('intensity-value').textContent = `${Math.round((next.fire.intensity || 0) * 100)}%`;
  $('ember-value').textContent = `${Math.round((next.fire.ember_heat || 0) * 100)}%`;
  $('tier-label').textContent = tr(`tier.${tier.toLowerCase()}Label`);
  $('phase-label').textContent = tr(`phase.${phase.toLowerCase()}`);
  $('source-count').textContent = `${next.sources.filter((source) => source.state === 'ok').length} ${tr('dynamic.connected')}`;
  const showRate = Boolean(configValue(config, 'showLiveRate', 'show_live_rate'));
  $('rate-chip').hidden = !showRate;
  $('rate-chip').textContent = `${(next.fire.tokens_per_second || 0).toFixed(1)} ${tr('dynamic.rate')}`;

  renderMix(next.today_by_source || []);
  renderSources(next.sources || []);
  renderBreakdown(next.breakdown || {});
  renderColorInputs();
  renderUpdate(next.update || { state: 'idle' });
  drawChart(next.hourly || []);
  drawWeekChart(next.last_seven_days || []);

  $('reduce-motion').checked = Boolean(configValue(config, 'reduceMotion', 'reduce_motion'));
  $('show-live-rate').checked = showRate;
  $('panel-visible').checked = Boolean(configValue(config, 'panelVisible', 'panel_visible'));
  $('token-poll-interval').value = String(configValue(config, 'tokenPollIntervalSeconds', 'token_poll_interval_seconds') || 2);
  $('animation-paused').checked = Boolean(next.fire.animation_paused);
  $('flame-size').value = configValue(config, 'flameSize', 'flame_size') || 'Medium';
  $('language').value = language;
  const paths = configValue(config, 'sourcePaths', 'source_paths') || [];
  const pathIndex = { ClaudeCode: 0, Codex: 1, Grok: 3, Pi: 4, Amp: 5 }[$('source-path-source').value];
  $('source-path-value').value = paths[pathIndex] || '';
  $('debug-tools').hidden = !Boolean(next.debug_tools_enabled ?? next.debugToolsEnabled);
}

function renderMix(values) {
  const total = values.reduce((a, b) => a + b, 0) || 1;
  $('mix-bar').replaceChildren(...values.map((value, i) => { const span = document.createElement('span'); span.style.width = `${(value / total) * 100}%`; span.style.background = sourceColors[i]; return span; }));
  $('mix-legend').replaceChildren(...values.map((value, i) => { const item = document.createElement('span'); item.className = 'legend-item'; const dot = document.createElement('i'); dot.className = 'legend-dot'; dot.style.background = sourceColors[i]; item.append(dot, `${sourceNames[i]} ${Math.round(value / total * 100)}%`); return item; }));
}

function renderSources(sources) {
  const detailKeys = {
    'No home directory': 'sources.noHome',
    'Pi logs not found': 'sources.piNotFound',
    'OpenCode uses an in-memory database': 'sources.openCodeMemory',
    'OpenCode database not found': 'sources.openCodeNotFound',
    'Cannot read OpenCode database': 'sources.openCodeUnreadable',
    'Unsupported OpenCode database schema': 'sources.openCodeUnsupported',
    'Missing Cursor state.vscdb': 'sources.cursorMissing',
    'Dashboard API ready': 'sources.cursorReady',
    'Local estimate (no Cursor token)': 'sources.cursorLocalNoToken',
    'Cursor not found': 'sources.cursorNotFound',
    'Dashboard API': 'sources.cursorDashboard',
    'Dashboard API (temporarily unavailable)': 'sources.cursorUnavailable',
    'Dashboard API (rate limited)': 'sources.cursorRateLimited',
    'Local estimate': 'sources.cursorLocal',
    'Amp logs not found': 'sources.ampNotFound',
    'Grok logs not found': 'sources.grokNotFound',
  };
  $('source-grid').replaceChildren(...sources.map((source, i) => {
    const card = document.createElement('article'); card.className = 'source-card';
    const stateKey = { ok: 'connected', notFound: 'notFound', noPermission: 'noPermission', unsupported: 'unsupported', readError: 'readError' }[source.state] || 'readError';
    card.innerHTML = `<div class="source-head"><span class="eyebrow"></span><span class="source-state ${source.state}"></span></div><h3></h3><p></p><small class="source-last-read"></small><div class="source-tokens"></div>`;
    card.querySelector('.eyebrow').textContent = `${tr('sources.source')} ${String(i + 1).padStart(2, '0')}`;
    card.querySelector('.source-state').textContent = tr(`sources.${stateKey}`);
    card.querySelector('h3').textContent = source.name;
    const detailKey = detailKeys[source.detail];
    card.querySelector('p').textContent = detailKey ? tr(detailKey) : source.detail && !['Connected', 'Not detected', 'No status detail'].includes(source.detail) ? source.detail : tr(source.state === 'ok' ? 'sources.connected' : `sources.${stateKey}`);
    const readAt = source.last_read_at || source.lastReadAt;
    card.querySelector('.source-last-read').textContent = readAt
      ? tr('sources.lastRead', { time: new Intl.DateTimeFormat(document.documentElement.lang || undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' }).format(new Date(readAt)) })
      : tr('sources.notReadYet');
    if (Number(source.estimated_tokens || 0) > 0) {
      const estimate = document.createElement('small');
      estimate.className = 'source-estimate';
      estimate.textContent = tr('sources.estimated', { tokens: formatTokens(source.estimated_tokens) });
      card.querySelector('.source-last-read').after(estimate);
    }
    card.querySelector('.source-tokens').textContent = `${formatTokens(source.tokens)} ${tr('sources.tokens')}`;
    return card;
  }));
}

function renderBreakdown(breakdown) {
  $('breakdown-input').textContent = breakdown.input == null ? '—' : formatTokens(breakdown.input);
  $('breakdown-output').textContent = breakdown.output == null ? '—' : formatTokens(breakdown.output);
  $('breakdown-cache-read').textContent = breakdown.cache_read == null ? '—' : formatTokens(breakdown.cache_read);
  $('breakdown-cache-write').textContent = breakdown.cache_write == null ? '—' : formatTokens(breakdown.cache_write);
}

function renderColorInputs() {
  if (document.activeElement?.type === 'color') return;
  $('color-grid').replaceChildren(...sourceNames.map((name, index) => {
    const label = document.createElement('label'); label.className = 'color-item';
    const input = document.createElement('input'); input.type = 'color'; input.value = sourceColors[index]; input.setAttribute('aria-label', `${name} ${tr('appearance.color')}`);
    input.addEventListener('change', () => invoke('set_source_color', { index, color: hexToRgb(input.value) }).then(refresh).catch(console.error));
    label.append(input, name); return label;
  }));
}

function renderUpdate(update) {
  const labels = {
    idle: tr('settings.updateReady'), checking: tr('update.checking'), upToDate: tr('update.upToDate'),
    available: tr('update.available', { version: update.version || '' }), incompatible: tr('update.incompatible', { version: update.version || '' }),
    downloading: tr('update.downloading', { version: update.version || '' }), ready: tr('update.ready', { version: update.version || '' }),
    failed: update.message || tr('update.failed'),
  };
  $('update-status').textContent = labels[update.state] || labels.idle;
  $('update-progress').hidden = update.state !== 'downloading' || update.progress == null; $('update-progress').value = update.progress || 0;
  $('download-update').hidden = update.state !== 'available'; $('install-update').hidden = update.state !== 'ready';
  $('dismiss-update').hidden = !['available', 'incompatible', 'ready', 'failed', 'upToDate'].includes(update.state); $('check-updates').hidden = ['checking', 'downloading'].includes(update.state);
}

function drawChart(points) {
  const canvas = $('chart-canvas'); const ctx = canvas.getContext('2d'); const w = canvas.width; const h = canvas.height; ctx.clearRect(0, 0, w, h); const max = Math.max(...points.map((point) => point.tokens), 1); const pad = 18; const gap = 7; const bw = (w - pad * 2 - gap * points.length) / Math.max(points.length, 1); let peak = { tokens: 0, hour: 0 };
  ctx.strokeStyle = 'rgba(255,238,224,.08)'; ctx.lineWidth = 1; for (let i = 1; i < 4; i++) { const y = pad + (h - pad * 2) * (i / 4); ctx.beginPath(); ctx.moveTo(pad, y); ctx.lineTo(w - pad, y); ctx.stroke(); }
  points.forEach((point, i) => { if (point.tokens > peak.tokens) peak = point; const bh = (point.tokens / max) * (h - pad * 2); const x = pad + i * (bw + gap); const grad = ctx.createLinearGradient(0, h - pad - bh, 0, h - pad); grad.addColorStop(0, '#ff9e61'); grad.addColorStop(1, 'rgba(214,72,44,.2)'); ctx.fillStyle = grad; ctx.beginPath(); ctx.roundRect(x, h - pad - bh, Math.max(1, bw), bh, 4); ctx.fill(); });
  $('peak-label').textContent = peak.tokens ? tr('dynamic.peak', { hour: peak.hour }) : '—';
}

function drawWeekChart(days) {
  const canvas = $('week-chart-canvas'); const ctx = canvas.getContext('2d'); const { width: w, height: h } = canvas;
  ctx.clearRect(0, 0, w, h);
  const pad = 24; const bottom = h - 32; const max = Math.max(...days.map((day) => day.tokens), 1);
  const gap = 18; const barWidth = Math.max(12, (w - pad * 2 - gap * Math.max(days.length - 1, 0)) / Math.max(days.length, 1));
  days.forEach((day, index) => {
    const x = pad + index * (barWidth + gap); const barHeight = Math.max(2, day.tokens / max * (h - 55));
    const gradient = ctx.createLinearGradient(0, bottom - barHeight, 0, bottom); gradient.addColorStop(0, '#ffb56f'); gradient.addColorStop(1, 'rgba(214,72,44,.24)');
    ctx.fillStyle = gradient; ctx.beginPath(); ctx.roundRect(x, bottom - barHeight, barWidth, barHeight, 4); ctx.fill();
    ctx.fillStyle = 'rgba(245,238,231,.62)'; ctx.font = '11px system-ui'; ctx.textAlign = 'center';
    const date = new Date(`${day.date}T12:00:00`); ctx.fillText(new Intl.DateTimeFormat(document.documentElement.lang || undefined, { weekday: 'short' }).format(date), x + barWidth / 2, h - 10);
  });
}

function exportShareCard() {
  if (!snapshot) return;
  const canvas = document.createElement('canvas'); canvas.width = 1200; canvas.height = 630;
  const ctx = canvas.getContext('2d'); const background = ctx.createLinearGradient(0, 0, 1200, 630);
  background.addColorStop(0, '#352018'); background.addColorStop(1, '#11100f'); ctx.fillStyle = background; ctx.fillRect(0, 0, 1200, 630);
  ctx.fillStyle = '#ffbd70'; ctx.font = '600 28px system-ui'; ctx.fillText('TokenBlaze', 78, 92);
  ctx.fillStyle = '#c18268'; ctx.font = '700 16px system-ui'; ctx.fillText(tr('overview.todayUsage'), 80, 190);
  ctx.fillStyle = '#fff3e5'; ctx.font = '600 94px system-ui'; ctx.fillText(formatTokens(snapshot.today_tokens), 76, 300);
  ctx.fillStyle = '#a99b92'; ctx.font = '24px system-ui'; ctx.fillText(tr('overview.tokensObserved'), 82, 346);
  ctx.fillStyle = '#e6a46f'; ctx.font = '20px system-ui'; ctx.fillText(new Intl.DateTimeFormat(document.documentElement.lang || undefined, { dateStyle: 'long' }).format(new Date()), 82, 410);
  const values = snapshot.today_by_source || []; const total = values.reduce((sum, value) => sum + value, 0) || 1;
  let x = 82; const y = 485; const barWidth = 1036;
  values.forEach((value, index) => { const width = value / total * barWidth; ctx.fillStyle = sourceColors[index]; ctx.fillRect(x, y, width, 14); x += width; });
  ctx.fillStyle = '#a99b92'; ctx.font = '16px system-ui'; ctx.fillText(tr('settings.privacyHint'), 82, 550, 1036);
  canvas.toBlob((blob) => {
    if (!blob) { showError(); return; }
    const url = URL.createObjectURL(blob); const link = document.createElement('a');
    link.href = url; link.download = `tokenblaze-${new Date().toISOString().slice(0, 10)}.png`; link.click(); URL.revokeObjectURL(url);
  }, 'image/png');
}

function showError() {
  const toast = $('error-toast');
  toast.textContent = tr('error.action');
  toast.hidden = false;
  clearTimeout(showError.timer);
  showError.timer = setTimeout(() => { toast.hidden = true; }, 5000);
}
const refreshIntervalMs = 1200;
const startupRetryIntervalMs = 250;
const startupGracePeriodMs = 5000;
const refreshStartedAt = Date.now();
let initialSnapshotReady = false;
async function refresh() {
  let nextRefreshDelay = refreshIntervalMs;
  try {
    updateSnapshot(await invoke('dashboard_snapshot'));
    initialSnapshotReady = true;
  } catch (error) {
    const waitingForStartup = !initialSnapshotReady && Date.now() - refreshStartedAt < startupGracePeriodMs;
    if (waitingForStartup) {
      nextRefreshDelay = startupRetryIntervalMs;
    } else {
      console.error('dashboard refresh failed', error);
      showError();
    }
  }
  window.setTimeout(refresh, nextRefreshDelay);
}
function run(command, args = {}) { return invoke(command, args).then(refresh).catch((error) => { console.error(`command failed: ${command}`, error); showError(); }); }

document.querySelectorAll('.nav-item').forEach((button) => button.addEventListener('click', () => { document.querySelectorAll('.nav-item').forEach((item) => item.classList.toggle('active', item === button)); document.querySelectorAll('.view').forEach((view) => view.classList.toggle('active', view.id === `view-${button.dataset.view}`)); applyLanguage(snapshot?.config?.language || 'English'); }));
$('reduce-motion').addEventListener('change', (event) => run('set_reduce_motion', { enabled: event.target.checked }));
$('show-live-rate').addEventListener('change', (event) => run('set_show_live_rate', { enabled: event.target.checked }));
$('token-poll-interval').addEventListener('change', (event) => run('set_token_poll_interval', { seconds: Number(event.target.value) }));
$('panel-visible').addEventListener('change', (event) => run('set_panel_visible', { visible: event.target.checked }));
$('animation-paused').addEventListener('change', (event) => run('set_animation_paused', { paused: event.target.checked }));
$('flame-size').addEventListener('change', (event) => run('set_flame_size', { size: event.target.value }));
$('language').addEventListener('change', (event) => run('set_language', { language: event.target.value }));
$('source-path-source').addEventListener('change', () => refresh());
$('save-source-path').addEventListener('click', async () => {
  try {
    await invoke('set_source_path', { source: $('source-path-source').value, path: $('source-path-value').value });
    await refresh();
  } catch (error) { console.error('source folder update failed', error); showError(); }
});
$('clear-source-path').addEventListener('click', async () => {
  try {
    await invoke('set_source_path', { source: $('source-path-source').value, path: null });
    $('source-path-value').value = '';
    await refresh();
  } catch (error) { console.error('source folder reset failed', error); showError(); }
});
$('reset-colors').addEventListener('click', () => run('reset_source_colors'));
$('share-card').addEventListener('click', exportShareCard);
$('show-preview').addEventListener('click', () => run('show_preview', { style: $('preview-style').value }));
$('return-live').addEventListener('click', () => run('return_to_live'));
$('inject-tokens').addEventListener('click', () => run('inject_tokens', { tokens: Number($('inject-value').value) || 0 }));
$('rescan').addEventListener('click', async () => {
  const button = $('rescan');
  button.disabled = true;
  button.setAttribute('aria-busy', 'true');
  button.querySelector('[data-i18n]').textContent = tr('settings.rescanWorking');
  try {
    await invoke('rescan');
    await refresh();
  } catch (error) {
    button.disabled = false;
    button.setAttribute('aria-busy', 'false');
    button.querySelector('[data-i18n]').textContent = tr('settings.rescanFailed');
    console.error('source rescan failed', error);
    showError();
  }
});
$('check-updates').addEventListener('click', () => run('check_updates'));
$('download-update').addEventListener('click', () => run('download_update'));
$('dismiss-update').addEventListener('click', () => run('dismiss_update'));
$('install-update').addEventListener('click', () => run('install_update'));
overviewFlame = window.HDFireRenderer.attach($('flame-canvas'));
refresh();
