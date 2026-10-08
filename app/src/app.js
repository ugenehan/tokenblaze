const $ = (id) => document.getElementById(id);
const sourceNames = ['Claude Code', 'Codex', 'Cursor', 'Grok', 'Pi', 'Amp', 'OpenCode'];
const builtInColors = ['#e8782e', '#47b86b', '#528feb', '#b86bf2', '#f29e38', '#33c7c7', '#ec4899'];
let sourceColors = [...builtInColors];
let snapshot = null;
let historyDays = 7;
let overviewRangeDays = 1;
let historyPoints = [];
let historyLoadedAt = 0;
let onboardingSeen = false;
let onboardingStep = 0;
let costConfigKey = '';
let costSummaryLoadedAt = 0;
let overviewFlame = null;
let appearanceFlame = null;
let mockTheme = 'Dark';
let mockLanguage = 'English';
let selectedTheme = 'Dark';
const systemTheme = window.matchMedia('(prefers-color-scheme: dark)');

const translationRows = [
  ['window.title', 'TokenBlaze Console', 'TokenBlaze 控制台', 'TokenBlaze コンソール', 'TokenBlaze 콘솔'],
  ['brand.subtitle', 'activity console', '活动控制台', 'アクティビティコンソール', '활동 콘솔'],
  ['status.local', 'Local only', '仅限本机', 'ローカルのみ', '로컬 전용'],
  ['nav.overview', 'Overview', '概览', '概要', '개요'],
  ['nav.sources', 'Sources', '数据源', 'データソース', '데이터 소스'],
  ['nav.appearance', 'Appearance', '外观', '炎の外観', '불꽃 모양'],
  ['nav.settings', 'Settings', '设置', '環境設定', '설정'],
  ['overview.heading', 'Today Overview', '今日概览', '今日の概要', '오늘 개요'],
  ['overview.live', 'Local monitoring', '本地守护中', 'ローカル監視中', '로컬 모니터링'],
  ['overview.heart', 'Activity Hearth', '活动火焰', '活動の炎', '활동 불꽃'],
  ['overview.range24h', '24 hours', '24小时', '24時間', '24시간'],
  ['overview.range7d', '7 days', '7天', '7日', '7일'],
  ['overview.range30d', '30 days', '30天', '30日', '30일'],
  ['overview.range90d', '90 days', '90天', '90日', '90일'],
  ['overview.exportMenu', 'Export data', '导出数据', 'データを保存', '데이터 내보내기'],
  ['overview.sessionStream', 'Recent usage events', '最近用量事件', '最近の利用イベント', '최근 사용량 이벤트'],
  ['overview.sessionHint', 'Latest locally recorded token usage', '最近在本机记录的 Token 用量', '端末内に記録された最新のトークン使用量', '최근 로컬 토큰 사용량 기록'],
  ['overview.noSessions', 'No usage events recorded yet.', '暂无用量事件记录。', '利用イベントはまだありません。', '사용량 이벤트 기록이 없습니다.'],
  ['overview.estimatedEvent', 'Estimated', '估算', '推定', '추정'],
  ['appearance.previewCanvas', 'Live flame companion preview', '实时火焰伴侣画布', '炎のライブプレビュー', '실시간 불꽃 미리보기'],
  ['appearance.floating', 'Floating', '悬浮', 'フローティング', '플로팅'],
  ['appearance.surfaceSection', 'Theme & Surface', '主题与界面', 'テーマと表示', '테마와 화면'],
  ['appearance.flameSection', 'Flame Companion', '火焰伴侣', '炎の動作', '불꽃 동작'],
  ['appearance.paletteSection', 'Source Color Palette', '数据源配色', 'ソースの色', '소스 색상'],
  ['settings.costSection', 'Cost estimates and budgets', '成本计费模型与预算提醒', '費用の推定と予算', '비용 추정 및 예산'],
  ['settings.dataSection', 'Local data and retention', '数据存储与隐私保留策略', 'ローカルデータと保存期間', '로컬 데이터 및 보존'],
  ['settings.systemSection', 'System and background', '系统与后台性能', 'システムとバックグラウンド', '시스템 및 백그라운드'],
  ['settings.updateSection', 'Updates and channels', '软件更新与渠道', '更新とチャンネル', '업데이트 및 채널'],
  ['header.liveActivity', 'LIVE ACTIVITY', '实时活动', 'ライブ活動', '실시간 활동'],
  ['header.console', 'TOKENBLAZE CONSOLE', 'TOKENBLAZE 控制台', 'TOKENBLAZE コンソール', 'TOKENBLAZE 콘솔'],
  ['header.core', 'TokenBlaze Core', 'TokenBlaze Core', 'TokenBlaze コア', 'TokenBlaze 코어'],
  ['header.live', 'LIVE', '实时', 'ライブ', '실시간'],
  ['overview.currentFlame', 'CURRENT FLAME', '当前火焰', '現在の炎', '현재 불꽃'],
  ['overview.intro', 'Live usage across your connected sources, shaped into one clear view.', '查看已连接数据源的实时用量概况。', '接続済みソースの使用状況をリアルタイムで確認できます。', '연결된 소스의 실시간 사용량을 한눈에 확인합니다.'],
  ['overview.fuel', 'fuel', '燃料', '燃料', '연료'],
  ['overview.intensity', 'intensity', '强度', '強度', '강도'],
  ['overview.emberHeat', 'ember heat', '余烬热度', '残り火', '잔불 열기'],
  ['overview.todayUsage', "TODAY'S USAGE", '今日用量', '今日の使用量', '오늘 사용량'],
  ['overview.tokensObserved', 'tokens observed', '已记录 Token 数', '観測トークン', '관측된 토큰'],
  ['overview.acrossSources', 'across 7 sources', '来自 7 个数据源', '7 個のソース', '7개 소스 전체'],
  ['overview.sourceMix', 'SOURCE MIX', '来源构成', 'ソース構成', '소스 구성'],
  ['overview.activityRhythm', 'ACTIVITY RHYTHM', '活动节奏', 'アクティビティ推移', '활동 흐름'],
  ['overview.last24Hours', 'Last 24 hours', '最近 24 小时', '過去 24 時間', '최근 24시간'],
  ['overview.todayByHour', 'Today by hour', '今日逐小时', '今日の時間別', '오늘 시간별'],
  ['overview.sevenDays', 'Last 7 days', '最近 7 天', '過去 7 日間', '최근 7일'],
  ['overview.thirtyDays', 'Last 30 days', '最近 30 天', '過去 30 日間', '최근 30일'],
  ['overview.ninetyDays', 'Last 90 days', '最近 90 天', '過去 90 日間', '최근 90일'],
  ['overview.historyRange', 'History range', '历史范围', '履歴の期間', '기록 기간'],
  ['overview.exportData', 'Export usage data', '导出用量数据', '利用データをエクスポート', '사용량 데이터 내보내기'],
  ['overview.exportHint', 'Local records only. Counts may be estimated; this is not a bill.', '仅导出本机记录。部分数据为估算值，不代表账单。', '端末内の記録のみ。推定値を含み、請求額ではありません。', '로컬 기록만 내보냅니다. 추정치가 포함되며 청구 금액이 아닙니다.'],
  ['overview.exportCsv', 'Export CSV', '导出 CSV', 'CSV を保存', 'CSV 내보내기'],
  ['overview.exportJson', 'Export JSON', '导出 JSON', 'JSON を保存', 'JSON 내보내기'],
  ['overview.emptyChart', 'Use a supported tool to see activity here.', '使用受支持的工具后，这里会显示活动曲线。', '対応ツールを使うと、ここに利用状況が表示されます。', '지원 도구를 사용하면 여기에 활동이 표시됩니다.'],
  ['overview.chartTokens', '{tokens} tokens', '{tokens} Token', '{tokens} トークン', '{tokens} 토큰'],
  ['overview.chartHour', '{hour}:00–{next}:00', '{hour}:00–{next}:00', '{hour}:00～{next}:00', '{hour}:00–{next}:00'],
  ['overview.hourlyUnit', 'tokens / hour', 'Token / 小时', 'トークン / 時間', '토큰 / 시간'],
  ['overview.dailyUnit', 'tokens / day', 'Token / 天', 'トークン / 日', '토큰 / 일'],
  ['overview.hourAxis', 'Time (hours)', '时间（小时）', '時刻（時間）', '시간（시）'],
  ['overview.dateAxis', 'Date', '日期', '日付', '날짜'],
  ['overview.hourlyHint', 'Hourly totals from local usage records', '按本机用量记录逐小时汇总', 'ローカルの利用記録を時間ごとに集計', '로컬 사용 기록의 시간별 합계'],
  ['overview.historyHint', 'Local usage totals for the selected period', '所选时间范围内的本机用量汇总', '選択した期間のローカル利用量', '선택한 기간의 로컬 사용량 합계'],
  ['overview.peakValue', 'Peak {tokens} at {hour}:00', '峰值 {tokens} · {hour}:00', 'ピーク {tokens}・{hour}:00', '최고 {tokens} · {hour}:00'],
  ['overview.exportPng', "Export today's card as PNG", '导出今日用量卡片 PNG', '今日の利用カードを PNG で保存', '오늘 사용량 카드를 PNG로 내보내기'],
  ['overview.tokenBreakdown', 'TOKEN BREAKDOWN', 'Token 明细', 'トークン内訳', '토큰 내역'],
  ['overview.whereUsageWent', 'Where usage went', '用量去向', '使用量の内訳', '사용량 구성'],
  ['stats.input', 'Input', '输入', '入力', '입력'],
  ['stats.output', 'Output', '输出', '出力', '출력'],
  ['stats.cacheRead', 'Cache read', '缓存读取', 'キャッシュ読み取り', '캐시 읽기'],
  ['stats.cacheWrite', 'Cache write', '缓存写入', 'キャッシュ書き込み', '캐시 쓰기'],
  ['sources.eyebrow', 'OBSERVABILITY', '可观测性', '可観測性', '관측성'],
  ['sources.title', 'Source connections', '本地客户端连接总览', 'データソース接続', '데이터 소스 연결'],
  ['sources.intro', 'Every source stays local. TokenBlaze reads usage events and turns them into one calm signal.', '所有数据都留在本机。TokenBlaze 读取用量事件，并将它们汇总为平稳的活动信号。', 'すべてのデータは端末内に留まります。TokenBlaze は使用イベントを読み取り、ひとつの穏やかな信号にまとめます。', '모든 데이터는 기기에만 남습니다. TokenBlaze는 사용 이벤트를 읽어 하나의 차분한 신호로 만듭니다.'],
  ['sources.source', 'SOURCE', '数据源', 'ソース', '소스'],
  ['sources.connected', 'CONNECTED', '已连接', '接続済み', '연결됨'],
  ['sources.lastRead', 'Last read: {time}', '最近读取：{time}', '最終読み取り: {time}', '마지막 읽기: {time}'],
  ['sources.notReadYet', 'No successful read yet', '尚未成功读取', 'まだ読み取りに成功していません', '아직 성공적으로 읽지 못함'],
  ['sources.estimated', 'Estimated: {tokens} tokens', '估算：{tokens} Token', '推定: {tokens} tokens', '추정: {tokens} tokens'],
  ['sources.notFound', 'NOT FOUND', '未发现', '未検出', '찾을 수 없음'],
  ['sources.noPermission', 'NO PERMISSION', '无权限', '権限なし', '권한 없음'],
  ['sources.unsupported', 'UNSUPPORTED', '不支持', '非対応', '지원 안 함'],
  ['sources.readError', 'READ ERROR', '读取错误', '読み取りエラー', '읽기 오류'],
  ['sources.noDetail', 'No status detail', '暂无状态详情', '状態の詳細はありません', '상태 세부 정보 없음'],
  ['sources.tokens', 'tokens', 'Token', 'トークン', '토큰'],
  ['sources.summary', '{connected} of {total} connected', '已连接 {connected}/{total} 个数据源', '{total} 件中 {connected} 件接続', '{total}개 중 {connected}개 연결됨'],
  ['sources.configurePath', 'Session folder', '会话目录', 'セッションフォルダー', '세션 폴더'],
  ['sources.pathInvalid', 'Choose an existing, readable folder.', '请选择存在且可读取的目录。', '存在し、読み取り可能なフォルダーを選択してください。', '존재하고 읽을 수 있는 폴더를 선택하세요.'],
  ['sources.noHome', 'No home directory', '未找到用户主目录', 'ホームディレクトリがありません', '홈 디렉토리를 찾을 수 없음'],
  ['sources.piNotFound', 'Pi logs not found', '未找到 Pi 日志', 'Pi のログが見つかりません', 'Pi 로그를 찾을 수 없음'],
  ['sources.openCodeMemory', 'OpenCode uses an in-memory database', 'OpenCode 正在使用内存数据库', 'OpenCode はメモリ内データベースを使用しています', 'OpenCode가 메모리 내 데이터베이스를 사용 중임'],
  ['sources.openCodeNotFound', 'OpenCode database not found', '未找到 OpenCode 数据库', 'OpenCode データベースが見つかりません', 'OpenCode 데이터베이스를 찾을 수 없음'],
  ['sources.openCodeUnreadable', 'Cannot read OpenCode database', '无法读取 OpenCode 数据库', 'OpenCode データベースを読み取れません', 'OpenCode 데이터베이스를 읽을 수 없음'],
  ['sources.openCodeUnsupported', 'Unsupported OpenCode database schema', '不支持的 OpenCode 数据库结构', '未対応の OpenCode データベース構造です', '지원하지 않는 OpenCode 데이터베이스 스키마'],
  ['sources.cursorMissing', 'Missing Cursor state.vscdb', '未找到 Cursor state.vscdb', 'Cursor の state.vscdb がありません', 'Cursor state.vscdb를 찾을 수 없음'],
  ['sources.cursorReady', 'Dashboard API ready', 'Dashboard API 已就绪', 'ダッシュボード API は準備完了です', '대시보드 API 준비됨'],
  ['sources.cursorLocalNoToken', 'Local estimate (no Cursor token)', '本地估算（无 Cursor 访问 Token）', 'ローカル推定（Cursor トークンなし）', '로컬 추정(Cursor 토큰 없음)'],
  ['sources.cursorNotFound', 'Cursor not found', '未找到 Cursor', 'Cursor が見つかりません', 'Cursor를 찾을 수 없음'],
  ['sources.cursorDashboard', 'Dashboard API', 'Dashboard API', 'ダッシュボード API', '대시보드 API'],
  ['sources.cursorUnavailable', 'Dashboard API (temporarily unavailable)', 'Dashboard API（暂时不可用）', 'ダッシュボード API（一時的に利用不可）', '대시보드 API(일시적으로 사용 불가)'],
  ['sources.cursorRateLimited', 'Dashboard API (rate limited)', 'Dashboard API（已限流）', 'ダッシュボード API（レート制限中）', '대시보드 API(요청 제한됨)'],
  ['sources.cursorLocal', 'Local estimate', '本地估算', 'ローカル推定', '로컬 추정'],
  ['sources.ampNotFound', 'Amp logs not found', '未找到 Amp 日志', 'Amp のログが見つかりません', 'Amp 로그를 찾을 수 없음'],
  ['sources.grokNotFound', 'Grok logs not found', '未找到 Grok 日志', 'Grok のログが見つかりません', 'Grok 로그를 찾을 수 없음'],
  ['appearance.eyebrow', 'VISUAL SYSTEM', '视觉系统', 'ビジュアルシステム', '비주얼 시스템'],
  ['appearance.theme', 'Theme', '主题', 'テーマ', '테마'],
  ['appearance.themeHint', 'Change the console and flame card colors.', '切换控制台和火焰信息卡的颜色。', 'コンソールと炎カードの色を切り替えます。', '콘솔과 불꽃 카드의 색상을 변경합니다.'],
  ['theme.system', 'Follow system', '跟随系统', 'システムに従う', '시스템 설정 따르기'],
  ['theme.dark', 'Dark', '深色', 'ダーク', '어둡게'],
  ['theme.light', 'Light', '浅色', 'ライト', '밝게'],
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
  ['settings.pollFrequency', 'Token polling frequency', 'Token 扫描频率', 'トークン取得間隔', '토큰 가져오기 간격'],
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
  ['settings.inject', 'Inject tokens', '注入 Token 用量', 'トークンを追加', '토큰 주입'],
  ['settings.updates', 'Updates', '更新', 'アップデート', '업데이트'],
  ['settings.updateReady', 'Ready to check.', '可以检查更新。', '確認できます。', '업데이트를 확인할 수 있습니다.'],
  ['settings.check', 'Check', '检查', '確認', '확인'],
  ['settings.download', 'Download', '下载', 'ダウンロード', '다운로드'],
  ['settings.install', 'Install & restart', '安装并重启', 'インストールして再起動', '설치 후 다시 시작'],
  ['settings.dismiss', 'Dismiss', '忽略', '閉じる', '닫기'],
  ['settings.privacy', 'Private by design', '隐私优先设计', 'プライバシー重視', '개인정보 보호 중심'],
  ['settings.privacyHint', "No usage data leaves this device. The console shares the native app's local store.", '用量数据不会离开此设备。控制台与原生应用共用本地存储。', '使用量データが端末外へ送信されることはありません。コンソールはネイティブアプリのローカルストアを共有します。', '사용량 데이터는 이 기기를 벗어나지 않습니다. 콘솔은 네이티브 앱의 로컬 저장소를 공유합니다.'],
  ['settings.dataRetention', 'Data retention', '数据保留', 'データ保持', '데이터 보관'],
  ['settings.dataRetentionHint', 'Older stored events are removed. Source logs are left untouched.', '过期的已存事件会被删除，原始会话日志不受影响。', '古い保存イベントを削除します。元のログは変更しません。', '오래된 저장 이벤트를 삭제합니다. 원본 로그는 유지됩니다.'],
  ['settings.forever', 'Forever', '永久', '無期限', '무기한'],
  ['settings.days', '{days} days', '{days} 天', '{days} 日', '{days}일'],
  ['settings.confirmRetention', 'Changing retention can permanently remove older stored usage. Continue?', '更改保留期可能永久删除较早的已存用量。继续吗？', '保持期間の変更により古い保存データが完全に削除される場合があります。続けますか？', '보관 기간을 변경하면 오래된 저장 사용량이 영구 삭제될 수 있습니다. 계속할까요?'],
  ['settings.clearUsage', 'Clear stored usage', '清除已存用量', '保存済み使用量を消去', '저장된 사용량 지우기'],
  ['settings.clearUsageHint', 'Deletes usage in TokenBlaze. Original source logs and preferences stay on this device.', '删除 TokenBlaze 中的用量；原始会话日志和偏好设置仍保留在本机。', 'TokenBlaze の使用量を削除します。元のログと設定は端末に残ります。', 'TokenBlaze의 사용량을 삭제합니다. 원본 로그와 설정은 기기에 남습니다.'],
  ['settings.confirmClear', 'Permanently delete all stored usage in TokenBlaze? This cannot be undone.', '永久删除 TokenBlaze 中所有已存用量？此操作无法撤销。', 'TokenBlaze に保存されたすべての使用量を完全に削除しますか？元に戻せません。', 'TokenBlaze에 저장된 모든 사용량을 영구 삭제할까요? 되돌릴 수 없습니다.'],
  ['onboarding.title', 'Welcome to TokenBlaze', '欢迎使用 TokenBlaze', 'TokenBlaze へようこそ', 'TokenBlaze에 오신 것을 환영합니다'],
  ['onboarding.step1', '1. Scan local sources', '1. 扫描本地数据源', '1. ローカルソースをスキャン', '1. 로컬 소스 검색'],
  ['onboarding.step1Hint', 'Open a supported tool, then scan for its local usage records.', '先打开受支持的工具，再扫描它的本机用量记录。', '対応ツールを開き、端末内の利用記録をスキャンします。', '지원 도구를 연 뒤 로컬 사용 기록을 검색하세요.'],
  ['onboarding.scan', 'Scan now', '立即扫描', '今すぐスキャン', '지금 검색'],
  ['onboarding.step2', '2. Review connections', '2. 查看连接结果', '2. 接続結果を確認', '2. 연결 결과 확인'],
  ['onboarding.step2Hint', 'Missing sources can be configured on the Sources page later.', '未发现的数据源可稍后在“数据源”页面配置目录。', '見つからないソースは後でソース画面で設定できます。', '찾을 수 없는 소스는 나중에 데이터 소스 페이지에서 설정할 수 있습니다.'],
  ['onboarding.step3', '3. Choose flame panel', '3. 选择火焰面板', '3. 炎パネルを選択', '3. 불꽃 패널 선택'],
  ['onboarding.step3Hint', 'The desktop flame can be changed later in Settings.', '之后仍可在设置中更改桌面火焰面板。', 'デスクトップの炎は後で設定から変更できます。', '데스크톱 불꽃은 나중에 설정에서 변경할 수 있습니다.'],
  ['onboarding.next', 'Next', '下一步', '次へ', '다음'],
  ['onboarding.finish', 'Finish', '完成', '完了', '완료'],
  ['onboarding.back', 'Back', '上一步', '戻る', '이전'],
  ['onboarding.skip', 'Skip', '跳过', 'スキップ', '건너뛰기'],
  ['onboarding.reopen', 'Show first-run guide', '重看首次引导', '初回ガイドを表示', '첫 사용 안내 다시 보기'],
  ['cost.title', 'Estimated cost', '估算成本', '推定コスト', '예상 비용'],
  ['cost.hint', 'Estimate from local tokens and your prices. Not a bill.', '按本机 Token 用量与自填单价估算，不代表账单。', '端末内のトークンと入力した単価による推定です。請求額ではありません。', '로컬 토큰과 입력한 단가로 계산한 추정치이며 청구 금액이 아닙니다.'],
  ['cost.disabled', 'Enable estimates and enter prices in Settings to show costs.', '在设置中启用估算并填写单价后显示费用。', '設定で推定を有効にし、単価を入力すると費用が表示されます。', '설정에서 추정을 켜고 단가를 입력하면 비용이 표시됩니다.'],
  ['cost.configure', 'Configure prices', '配置估算单价', '単価を設定', '단가 설정'],
  ['cost.enable', 'Show cost estimate', '显示成本估算', 'コスト推定を表示', '비용 추정치 표시'],
  ['cost.rates', 'USD per million tokens', '单价：USD / 百万 Token', '単価：100万トークンあたり USD', '단가: 백만 토큰당 USD'],
  ['cost.ratesHint', 'Enter only the prices you know. Empty fields remain unestimated.', '只填写已知单价；留空的用量不会计入估算。', '分かる単価だけ入力してください。空欄の使用量は推定に含まれません。', '알고 있는 단가만 입력하세요. 빈칸의 사용량은 추정에 포함되지 않습니다.'],
  ['cost.today', 'Today', '今日', '今日', '오늘'],
  ['cost.week', 'Last 7 days', '最近 7 天', '過去 7 日間', '최근 7일'],
  ['cost.dailyBudget', 'Daily budget (USD)', '每日预算（USD）', '1日の予算（USD）', '일일 예산(USD)'],
  ['cost.weeklyBudget', '7-day budget (USD)', '7 天预算（USD）', '7日間の予算（USD）', '7일 예산(USD)'],
  ['cost.save', 'Save cost settings', '保存成本设置', 'コスト設定を保存', '비용 설정 저장'],
  ['cost.confirmEnable', 'Show estimates based on your prices? These are not billing amounts.', '按您填写的单价显示估算值？它们不是实际账单金额。', '入力した単価で推定値を表示しますか？請求額ではありません。', '입력한 단가로 추정치를 표시할까요? 실제 청구 금액이 아닙니다.'],
  ['cost.partial', 'Partial estimate: some usage has no price or token breakdown.', '仅为部分估算：部分用量缺少单价或 Token 分类。', '一部の推定値です。単価またはトークン内訳がありません。', '일부 추정치입니다. 단가 또는 토큰 내역이 없습니다.'],
  ['cost.budgetAlert', 'Estimated cost reached a budget threshold.', '估算成本已达到预算阈值。', '推定コストが予算に達しました。', '예상 비용이 예산 한도에 도달했습니다.'],
  ['sources.disconnected', 'This source was connected before but failed three checks. Check its local logs and permissions, then rescan.', '此来源曾连接，但连续三次检查失败。请检查本机日志和权限，然后重新扫描。', '以前接続されたソースが3回連続で失敗しました。ローカルログと権限を確認し、再スキャンしてください。', '이전에 연결된 소스가 세 번 연속 실패했습니다. 로컬 로그와 권한을 확인한 후 다시 검색하세요.'],
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
  ['dynamic.active', 'active', '个活跃', '件有効', '개 활성'], ['dynamic.rate', 'token/s · est.', 'Token/s · 估算', 'token/s · 推定', 'token/s · 추정'],
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
  ['update.openReleases', 'Open Releases page', '打开版本发布页', 'リリースページを開く', '릴리스 페이지 열기'],
];
const languageNames = ['English', 'Chinese', 'Japanese', 'Korean'];
const copy = Object.fromEntries(languageNames.map((language, languageIndex) => [language, Object.fromEntries(translationRows.map((row) => [row[0], row[languageIndex + 1]]))]));
let selectedLanguage = 'English';

async function invoke(command, args = {}) {
  const api = window.__TAURI__?.core?.invoke;
  if (api) return api(command, args);
  if (command === 'set_theme') { mockTheme = args.theme; return undefined; }
  if (command === 'set_language') { mockLanguage = args.language; return undefined; }
  if (command === 'dashboard_snapshot') return mockSnapshot();
  if (command === 'usage_history') return mockHistory(args.days);
  return undefined;
}

function mockHistory(days) {
  return Array.from({ length: days }, (_, i) => ({
    date: new Date(Date.now() - (days - 1 - i) * 86400000).toISOString().slice(0, 10),
    tokens: Math.round(9000 + Math.sin(i * .8) * 4500 + Math.cos(i * .29) * 2500),
  }));
}

function mockSnapshot() {
  const t = Date.now() / 1000;
  const hourly = Array.from({ length: 24 }, (_, hour) => ({ hour, tokens: Math.round(Math.max(0, Math.sin((hour - 4) / 3) * 1600 + Math.random() * 420)) }));
  return {
    today_tokens: 18240,
    today_by_source: [8200, 4200, 2500, 1600, 900, 540, 300],
    hourly,
    last_seven_days: mockHistory(7),
    breakdown: { input: 10100, output: 5340, cache_read: 2100, cache_write: 700 },
    fire: { intensity: .64 + Math.sin(t) * .08, fuel: .72, ember_heat: .45, spark_burst: .2, phase: 'flame', tier: 'crackle', color_mix: [.5, .25, .1, .05, .04, .03, .03], tokens_per_second: 24.8, animation_paused: false, previewing: false },
    sources: sourceNames.map((name, i) => ({ id: name.toLowerCase(), name, tokens: [8200, 4200, 2500, 1600, 900, 540, 300][i], state: i < 5 ? 'ok' : 'notFound', detail: i < 5 ? 'Connected' : 'Not detected' })),
    debug_tools_enabled: false,
    config: { soundEnabled: false, soundVolume: .48, reduceMotion: false, showLiveRate: true, tokenPollIntervalSeconds: 2, panelVisible: true, language: mockLanguage, theme: mockTheme, flameSize: 'Medium', sourceColors: builtInColors.map(hexToRgb) },
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
const themeRow = document.createElement('div');
themeRow.className = 'setting-row';
themeRow.innerHTML = '<div><strong data-i18n="appearance.theme"></strong><small data-i18n="appearance.themeHint"></small></div><select id="theme"><option value="System" data-i18n="theme.system"></option><option value="Dark" data-i18n="theme.dark"></option><option value="Light" data-i18n="theme.light"></option></select>';
document.querySelector('#view-appearance .settings-card').prepend(themeRow);
const scrollbarStyle = document.createElement('style');
scrollbarStyle.textContent = '*{scrollbar-width:thin;scrollbar-color:rgba(197,174,160,.05) transparent}*:hover{scrollbar-color:rgba(197,174,160,.2) transparent}::-webkit-scrollbar{width:6px;height:6px}::-webkit-scrollbar-track{background:transparent}::-webkit-scrollbar-thumb{background:rgba(197,174,160,.045);border:2px solid transparent;background-clip:padding-box;border-radius:99px;transition:background-color .18s ease}*:hover::-webkit-scrollbar-thumb{background-color:rgba(197,174,160,.2)}::-webkit-scrollbar-thumb:hover{background-color:rgba(232,166,108,.42)}';
document.head.append(scrollbarStyle);

function applyLanguage(language) {
  const previousLanguage = selectedLanguage;
  selectedLanguage = effectiveLanguage(language);
  if (previousLanguage !== selectedLanguage) costSummaryLoadedAt = 0;
  document.documentElement.lang = { Chinese: 'zh-CN', Japanese: 'ja', Korean: 'ko' }[selectedLanguage] || 'en';
  document.title = tr('window.title');
  document.querySelectorAll('[data-i18n]').forEach((element) => { element.textContent = tr(element.dataset.i18n, element.dataset.days ? { days: element.dataset.days } : {}); });
  document.querySelectorAll('[data-i18n-aria-label]').forEach((element) => { element.setAttribute('aria-label', tr(element.dataset.i18nAriaLabel)); });
  document.querySelectorAll('.nav-item').forEach((button) => { button.setAttribute('aria-label', tr(`nav.${button.dataset.view}`)); });
  $('chart-canvas').setAttribute('aria-label', tr('overview.todayByHour'));
  $('week-chart-canvas').setAttribute('aria-label', tr('overview.sevenDays'));
  $('history-range').setAttribute('aria-label', tr('overview.historyRange'));
  $('history-title').textContent = tr({ 7: 'overview.sevenDays', 30: 'overview.thirtyDays', 90: 'overview.ninetyDays' }[historyDays]);
  $('week-chart-canvas').setAttribute('aria-label', $('history-title').textContent);
  $('retention-days')?.setAttribute('aria-label', tr('settings.dataRetention'));
  document.querySelector('.retention-choices')?.setAttribute('aria-label', tr('settings.dataRetention'));
  document.querySelector('.theme-choices')?.setAttribute('aria-label', tr('appearance.theme'));
  $('onboarding-next').textContent = tr(onboardingStep === 2 ? 'onboarding.finish' : 'onboarding.next');
  $('cost-today-label').textContent = tr('cost.today');
  $('cost-week-label').textContent = tr('cost.week');
  if (snapshot) { drawChart(snapshot.hourly || []); drawWeekChart(historyPoints); }
}

function applyTheme(theme) {
  selectedTheme = ['System', 'Dark', 'Light'].includes(theme) ? theme : 'Dark';
  document.documentElement.dataset.theme = selectedTheme === 'System'
    ? (systemTheme.matches ? 'dark' : 'light') : selectedTheme.toLowerCase();
  $('theme').value = selectedTheme;
  document.querySelectorAll('[data-theme-choice]').forEach((button) => {
    const active = button.dataset.themeChoice === selectedTheme;
    button.classList.toggle('active', active);
    button.setAttribute('aria-pressed', String(active));
  });
}
systemTheme.addEventListener('change', () => {
  if (selectedTheme === 'System') {
    applyTheme(selectedTheme);
    if (snapshot) { drawChart(snapshot.hourly || []); drawWeekChart(historyPoints); }
  }
});

function updateSnapshot(next) {
  snapshot = next;
  const config = next.config || {};
  overviewFlame?.setState({ fire: next.fire, reduceMotion: Boolean(configValue(config, 'reduceMotion', 'reduce_motion')) });
  appearanceFlame?.setState({ fire: next.fire, reduceMotion: Boolean(configValue(config, 'reduceMotion', 'reduce_motion')) });
  const language = config.language || 'English';
  applyLanguage(language);
  applyTheme(config.theme || 'Dark');
  const rescanButton = $('rescan');
  rescanButton.disabled = Boolean(next.is_rescanning);
  rescanButton.setAttribute('aria-busy', String(Boolean(next.is_rescanning)));
  rescanButton.querySelector('[data-i18n]').textContent = tr(next.is_rescanning ? 'settings.rescanWorking' : 'settings.rescan');
  const phase = enumTitle(next.fire.phase);
  const tier = enumTitle(next.fire.tier);
  const colors = configValue(config, 'sourceColors', 'source_colors');
  if (Array.isArray(colors) && colors.length === 7) sourceColors = colors.map(rgbToHex);

  $('token-value').textContent = formatTokens(next.today_tokens);
  $('topbar-total').textContent = `${formatTokens(next.today_tokens)} ${tr('sources.tokens')}`;
  $('poll-status').textContent = `${configValue(config, 'tokenPollIntervalSeconds', 'token_poll_interval_seconds') || 2}s`;
  $('fuel-value').textContent = `${Math.round((next.fire.fuel || 0) * 100)}%`;
  $('intensity-value').textContent = `${Math.round((next.fire.intensity || 0) * 100)}%`;
  $('ember-value').textContent = `${Math.round((next.fire.ember_heat || 0) * 100)}%`;
  $('tier-label').textContent = tr(`tier.${tier.toLowerCase()}Label`);
  $('phase-label').textContent = tr(`phase.${phase.toLowerCase()}`);
  $('source-count').textContent = `${next.sources.filter((source) => source.state === 'ok').length} ${tr('dynamic.connected')}`;
  const showRate = Boolean(configValue(config, 'showLiveRate', 'show_live_rate'));
  $('rate-chip').hidden = !showRate;
  $('rate-chip').textContent = `${(next.fire.tokens_per_second || 0).toFixed(1)} ${tr('dynamic.rate')}`;
  $('preview-rate').textContent = `${(next.fire.tokens_per_second || 0).toFixed(1)} ${tr('dynamic.rate')}`;
  $('preview-tier').textContent = tr(`tier.${tier.toLowerCase()}Label`);
  $('preview-intensity').textContent = `${Math.round((next.fire.intensity || 0) * 100)}%`;

  renderMix(next.today_by_source || []);
  renderSources(next.sources || [], configValue(config, 'sourcePaths', 'source_paths') || []);
  renderRecentActivity(next.recent_events || []);
  renderBreakdown(next.breakdown || {});
  renderColorInputs();
  renderUpdate(next.update || { state: 'idle' });
  drawChart(next.hourly || []);
  if (historyDays === 7) historyPoints = next.last_seven_days || [];
  drawWeekChart(historyPoints);

  $('reduce-motion').checked = Boolean(configValue(config, 'reduceMotion', 'reduce_motion'));
  $('show-live-rate').checked = showRate;
  $('panel-visible').checked = Boolean(configValue(config, 'panelVisible', 'panel_visible'));
  $('token-poll-interval').value = String(configValue(config, 'tokenPollIntervalSeconds', 'token_poll_interval_seconds') || 2);
  $('retention-days').value = String(configValue(config, 'retentionDays', 'retention_days') ?? 0);
  document.querySelectorAll('[data-retention-choice]').forEach((button) => {
    const active = button.dataset.retentionChoice === $('retention-days').value;
    button.classList.toggle('active', active);
    button.setAttribute('aria-pressed', String(active));
  });
  $('onboarding-result').textContent = tr('sources.summary', { connected: next.sources.filter((source) => source.state === 'ok').length, total: next.sources.length });
  const costEnabled = Boolean(configValue(config, 'costEnabled', 'cost_enabled'));
  $('cost-card').hidden = false;
  $('cost-card').classList.toggle('estimate-disabled', !costEnabled);
  document.querySelector('.topbar-separator').hidden = !costEnabled;
  $('topbar-cost').hidden = !costEnabled;
  if (!costEnabled) {
    $('cost-today').textContent = '—'; $('cost-week').textContent = '—'; $('topbar-cost').textContent = '—';
    $('cost-status').dataset.i18n = 'cost.disabled';
    $('cost-status').textContent = tr('cost.disabled');
  }
  const nextCostConfigKey = JSON.stringify([costEnabled, configValue(config, 'costRates', 'cost_rates'), configValue(config, 'dailyBudgetUsd', 'daily_budget_usd'), configValue(config, 'weeklyBudgetUsd', 'weekly_budget_usd')]);
  if (costConfigKey !== nextCostConfigKey) {
    costConfigKey = nextCostConfigKey;
    $('cost-enabled').checked = costEnabled;
    const rates = configValue(config, 'costRates', 'cost_rates') || [];
    document.querySelectorAll('.cost-rate').forEach((input) => { input.value = rates[Number(input.dataset.source)]?.[input.dataset.kind] ?? ''; });
    $('cost-daily-budget').value = configValue(config, 'dailyBudgetUsd', 'daily_budget_usd') ?? '';
    $('cost-weekly-budget').value = configValue(config, 'weeklyBudgetUsd', 'weekly_budget_usd') ?? '';
    costSummaryLoadedAt = 0;
  }
  if (configValue(config, 'onboardingComplete', 'onboarding_complete') === false && !onboardingSeen) {
    onboardingSeen = true;
    $('onboarding-panel').checked = Boolean(configValue(config, 'panelVisible', 'panel_visible'));
    showOnboarding(0);
  }
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
  $('mix-legend').replaceChildren(...values.map((value, i) => {
    const item = document.createElement('span'); item.className = 'legend-item';
    const heading = document.createElement('span'); heading.className = 'legend-heading';
    const dot = document.createElement('i'); dot.className = 'legend-dot'; dot.style.background = sourceColors[i];
    const name = document.createElement('span'); name.className = 'legend-name'; name.textContent = sourceNames[i];
    const share = document.createElement('span'); share.className = 'legend-share'; share.textContent = `${Math.round(value / total * 100)}%`;
    const amount = document.createElement('strong'); amount.className = 'legend-value'; amount.textContent = `${formatTokens(value)} ${tr('sources.tokens')}`;
    heading.append(dot, name, share); item.append(heading, amount); return item;
  }));
}

function renderSources(sources, paths) {
  const connected = sources.filter((source) => source.state === 'ok').length;
  $('source-summary').textContent = tr('sources.summary', { connected, total: sources.length });
  $('nav-source-count').textContent = `${connected}/${sources.length}`;
  document.querySelector('.first-run-note').hidden = connected > 0;
  if ($('source-grid').querySelector('details[open]')) return;
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
    card.innerHTML = `<div class="source-head"><span class="source-icon">✦</span><div class="source-ident"><h3></h3><small></small></div><span class="source-state ${source.state}"></span></div><div class="source-usage"><small></small><div class="source-tokens"></div></div><p></p><small class="source-last-read"></small>`;
    card.querySelector('.source-icon').style.color = sourceColors[i];
    card.querySelector('.source-ident small').textContent = `${tr('sources.source')} ${String(i + 1).padStart(2, '0')}`;
    card.querySelector('.source-usage small').textContent = tr('overview.todayUsage');
    card.querySelector('.source-state').textContent = tr(`sources.${stateKey}`);
    card.querySelector('h3').textContent = source.name;
    const detailKey = detailKeys[source.detail];
    const detail = detailKey ? tr(detailKey) : source.detail && !['Connected', 'Not detected', 'No status detail'].includes(source.detail) ? source.detail : '';
    card.querySelector('p').textContent = detail;
    card.querySelector('p').hidden = !detail;
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
    const pathSource = { 0: 'ClaudeCode', 1: 'Codex', 3: 'Grok', 4: 'Pi', 5: 'Amp' }[i];
    if (source.needs_connection_help || source.needsConnectionHelp) {
      const repair = document.createElement('div'); repair.className = 'source-repair';
      const hint = document.createElement('p'); hint.textContent = tr('sources.disconnected');
      const retry = document.createElement('button'); retry.className = 'text-button'; retry.textContent = tr('settings.rescan');
      retry.addEventListener('click', () => invoke('rescan').catch(showError));
      repair.append(hint, retry); card.append(repair);
    }
    if (pathSource) {
      const details = document.createElement('details'); details.className = 'source-path-inline';
      const summary = document.createElement('summary');
      const summaryLabel = document.createElement('span'); summaryLabel.textContent = tr('sources.configurePath');
      const summaryPath = document.createElement('code'); summaryPath.textContent = paths[i] || tr('settings.clearPath'); summaryPath.title = summaryPath.textContent;
      summary.append(summaryLabel, summaryPath);
      const input = document.createElement('input'); input.type = 'text'; input.value = paths[i] || ''; input.setAttribute('aria-label', `${source.name} ${tr('sources.configurePath')}`);
      const actions = document.createElement('div'); actions.className = 'path-actions';
      const save = document.createElement('button'); save.className = 'action-button compact'; save.textContent = tr('settings.savePath');
      const automatic = document.createElement('button'); automatic.className = 'text-button'; automatic.textContent = tr('settings.clearPath');
      const error = document.createElement('p'); error.className = 'path-error'; error.setAttribute('role', 'alert'); error.hidden = true;
      save.addEventListener('click', async () => {
        try { await invoke('set_source_path', { source: pathSource, path: input.value }); details.open = false; updateSnapshot(await invoke('dashboard_snapshot')); }
        catch { error.textContent = tr('sources.pathInvalid'); error.hidden = false; input.setAttribute('aria-invalid', 'true'); }
      });
      automatic.addEventListener('click', async () => {
        try { await invoke('set_source_path', { source: pathSource, path: null }); details.open = false; updateSnapshot(await invoke('dashboard_snapshot')); }
        catch { error.textContent = tr('sources.pathInvalid'); error.hidden = false; }
      });
      input.addEventListener('input', () => { error.hidden = true; input.removeAttribute('aria-invalid'); });
      actions.append(save, automatic); details.append(summary, input, actions, error); card.append(details);
    }
    return card;
  }));
}

function renderBreakdown(breakdown) {
  $('breakdown-input').textContent = breakdown.input == null ? '—' : formatTokens(breakdown.input);
  $('breakdown-output').textContent = breakdown.output == null ? '—' : formatTokens(breakdown.output);
  $('breakdown-cache-read').textContent = breakdown.cache_read == null ? '—' : formatTokens(breakdown.cache_read);
  $('breakdown-cache-write').textContent = breakdown.cache_write == null ? '—' : formatTokens(breakdown.cache_write);
}

function renderRecentActivity(events) {
  const stream = $('session-stream');
  if (!events.length) { stream.textContent = tr('overview.noSessions'); return; }
  stream.replaceChildren(...events.slice(0, 3).map((event) => {
    const row = document.createElement('div'); row.className = 'session-row';
    const icon = document.createElement('span'); icon.className = 'session-icon'; icon.style.color = sourceColors[event.source_index] || sourceColors[0]; icon.textContent = '✦';
    const name = document.createElement('strong'); name.textContent = event.source;
    const info = document.createElement('div'); info.className = 'session-info'; info.append(name);
    if (event.is_estimated) {
      const estimate = document.createElement('span'); estimate.className = 'session-estimate'; estimate.textContent = tr('overview.estimatedEvent'); info.append(estimate);
    }
    const amount = document.createElement('strong'); amount.textContent = `+${formatTokens(event.tokens)} ${tr('sources.tokens')}`;
    const time = document.createElement('small'); time.textContent = new Intl.DateTimeFormat(document.documentElement.lang || undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(new Date(event.timestamp));
    const value = document.createElement('div'); value.className = 'session-value'; value.append(amount, time);
    row.append(icon, info, value); return row;
  }));
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
  $('open-releases').hidden = !['failed', 'incompatible'].includes(update.state);
}

function prepareChartCanvas(canvas) {
  const w = Math.max(1, Math.round(canvas.clientWidth || canvas.chartCssWidth || 760));
  const h = Math.max(200, Math.round(canvas.clientHeight || canvas.chartCssHeight || 270));
  if (canvas.clientWidth) { canvas.chartCssWidth = w; canvas.chartCssHeight = h; }
  const scale = Math.min(window.devicePixelRatio || 1, 2);
  if (canvas.width !== Math.round(w * scale) || canvas.height !== Math.round(h * scale)) {
    canvas.width = Math.round(w * scale);
    canvas.height = Math.round(h * scale);
  }
  const ctx = canvas.getContext('2d');
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.clearRect(0, 0, w, h);
  return { ctx, w, h };
}

function chartScale(points) {
  const max = Math.max(...points.map((point) => point.tokens), 1);
  const magnitude = 10 ** Math.floor(Math.log10(max / 4));
  const step = [1, 2, 2.5, 5, 10].find((value) => value * magnitude >= max / 4) * magnitude;
  return step * 4;
}

function drawChartAxes(ctx, plot, max, unit, xUnit, h) {
  const style = getComputedStyle(document.documentElement);
  const label = style.getPropertyValue('--chart-label');
  ctx.font = '11px system-ui';
  ctx.fillStyle = label;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(unit, plot.left, 16);
  ctx.strokeStyle = style.getPropertyValue('--chart-grid');
  ctx.lineWidth = 1;
  for (let tick = 0; tick <= 4; tick++) {
    const y = plot.bottom - tick * (plot.bottom - plot.top) / 4;
    ctx.beginPath(); ctx.moveTo(plot.left, y + .5); ctx.lineTo(plot.right, y + .5); ctx.stroke();
    ctx.textAlign = 'right';
    ctx.fillText(new Intl.NumberFormat(document.documentElement.lang || undefined, { notation: max >= 1000 ? 'compact' : 'standard', maximumFractionDigits: 1 }).format(max * tick / 4), plot.left - 10, y + 4);
  }
  ctx.textAlign = 'center';
  ctx.fillText(xUnit, (plot.left + plot.right) / 2, h - 7);
}

function drawChart(points) {
  $('hour-empty').hidden = points.some((point) => point.tokens > 0);
  const canvas = $('chart-canvas');
  canvas.setAttribute('aria-label', `${tr('overview.todayByHour')} · ${tr('overview.hourlyUnit')} · ${tr('overview.hourAxis')}`);
  const { ctx, w, h } = prepareChartCanvas(canvas);
  const plot = { left: 60, right: w - 22, top: 35, bottom: h - 55 };
  const max = chartScale(points);
  drawChartAxes(ctx, plot, max, tr('overview.hourlyUnit'), tr('overview.hourAxis'), h);
  const step = (plot.right - plot.left) / Math.max(points.length - 1, 1);
  const positions = points.map((point, index) => ({
    x: plot.left + index * step,
    y: plot.bottom - point.tokens / max * (plot.bottom - plot.top),
  }));
  canvas.chartPositions = positions.map((position) => position.x);
  canvas.chartPlot = plot;
  if (points.length) {
    const barWidth = Math.min(14, step * .32);
    ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--chart-bar');
    positions.forEach(({ x, y }) => {
      if (y >= plot.bottom) return;
      ctx.beginPath(); ctx.roundRect(x - barWidth / 2, y, barWidth, plot.bottom - y, 2); ctx.fill();
    });
    const trace = () => {
      ctx.beginPath(); ctx.moveTo(positions[0].x, positions[0].y);
      for (let i = 1; i < positions.length; i++) {
        const previous = positions[i - 1]; const current = positions[i];
        const handle = (current.x - previous.x) * .42;
        ctx.bezierCurveTo(previous.x + handle, previous.y, current.x - handle, current.y, current.x, current.y);
      }
    };
    const fill = ctx.createLinearGradient(0, plot.top, 0, plot.bottom);
    fill.addColorStop(0, 'rgba(255, 181, 150, .30)');
    fill.addColorStop(1, 'rgba(240, 106, 37, .015)');
    trace(); ctx.lineTo(positions[positions.length - 1].x, plot.bottom);
    ctx.lineTo(positions[0].x, plot.bottom); ctx.closePath();
    ctx.fillStyle = fill; ctx.fill();
    trace();
    const stroke = ctx.createLinearGradient(plot.left, 0, plot.right, 0);
    stroke.addColorStop(0, '#f06a25'); stroke.addColorStop(.55, '#ffad82'); stroke.addColorStop(1, '#ffdbce');
    ctx.strokeStyle = stroke; ctx.lineWidth = 2.5; ctx.lineCap = 'round'; ctx.stroke();
    const peakIndex = points.reduce((best, point, index) => point.tokens > points[best].tokens ? index : best, 0);
    const peak = points[peakIndex];
    if (peak.tokens > 0) {
      const { x, y } = positions[peakIndex];
      ctx.fillStyle = 'rgba(255, 181, 150, .19)';
      ctx.beginPath(); ctx.arc(x, y, 10, 0, Math.PI * 2); ctx.fill();
      ctx.fillStyle = '#ffb596';
      ctx.beginPath(); ctx.arc(x, y, 4.5, 0, Math.PI * 2); ctx.fill();
      $('peak-label').textContent = tr('overview.peakValue', { tokens: formatTokens(peak.tokens), hour: String(peak.hour).padStart(2, '0') });
    } else {
      $('peak-label').textContent = '—';
    }
  } else {
    $('peak-label').textContent = '—';
  }
  ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--chart-label');
  ctx.font = '11px system-ui'; ctx.textBaseline = 'top'; ctx.textAlign = 'center';
  (w < 480 ? [0, 6, 12, 18, 23] : [0, 4, 8, 12, 16, 20, 23]).forEach((hour) => {
    const index = points.findIndex((point) => point.hour === hour);
    const x = index >= 0 ? positions[index].x : plot.left + hour / 23 * (plot.right - plot.left);
    ctx.fillText(`${String(hour).padStart(2, '0')}:00`, x, plot.bottom + 12);
  });
}

function drawWeekChart(days) {
  $('week-empty').hidden = days.some((day) => day.tokens > 0);
  const canvas = $('week-chart-canvas');
  canvas.setAttribute('aria-label', `${$('history-title').textContent} · ${tr('overview.dailyUnit')} · ${tr('overview.dateAxis')}`);
  const { ctx, w, h } = prepareChartCanvas(canvas);
  const plot = { left: 60, right: w - 22, top: 35, bottom: h - 55 };
  const max = chartScale(days);
  drawChartAxes(ctx, plot, max, tr('overview.dailyUnit'), tr('overview.dateAxis'), h);
  const step = (plot.right - plot.left) / Math.max(days.length, 1);
  const barWidth = Math.min(days.length > 14 ? 22 : 55, step * .65);
  canvas.chartPositions = days.map((_, index) => plot.left + step * (index + .5));
  canvas.chartPlot = plot;
  days.forEach((day, index) => {
    const x = canvas.chartPositions[index]; const barHeight = day.tokens / max * (plot.bottom - plot.top);
    const gradient = ctx.createLinearGradient(0, plot.bottom - barHeight, 0, plot.bottom);
    gradient.addColorStop(0, '#ffb596'); gradient.addColorStop(1, 'rgba(240, 106, 37, .28)');
    ctx.fillStyle = gradient;
    ctx.beginPath(); ctx.roundRect(x - barWidth / 2, plot.bottom - barHeight, barWidth, Math.max(barHeight, 1), 4); ctx.fill();
    ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--chart-label'); ctx.font = '11px system-ui'; ctx.textAlign = 'center';
    if (days.length <= 7 || index % Math.max(1, Math.ceil(days.length / 7)) === 0 || index === days.length - 1) {
      const date = new Date(`${day.date}T12:00:00`);
      const options = { month: 'numeric', day: 'numeric' };
      ctx.textBaseline = 'top';
      ctx.fillText(new Intl.DateTimeFormat(document.documentElement.lang || undefined, options).format(date), x, plot.bottom + 12);
    }
  });
  if (days.length) {
    const positions = days.map((day, index) => ({
      x: canvas.chartPositions[index],
      y: plot.bottom - day.tokens / max * (plot.bottom - plot.top),
    }));
    const trace = () => {
      ctx.beginPath(); ctx.moveTo(positions[0].x, positions[0].y);
      for (let i = 1; i < positions.length; i++) {
        const previous = positions[i - 1]; const current = positions[i];
        const handle = (current.x - previous.x) * .42;
        ctx.bezierCurveTo(previous.x + handle, previous.y, current.x - handle, current.y, current.x, current.y);
      }
    };
    const fill = ctx.createLinearGradient(0, plot.top, 0, plot.bottom);
    fill.addColorStop(0, 'rgba(255, 181, 150, .30)');
    fill.addColorStop(1, 'rgba(240, 106, 37, .015)');
    trace(); ctx.lineTo(positions[positions.length - 1].x, plot.bottom);
    ctx.lineTo(positions[0].x, plot.bottom); ctx.closePath();
    ctx.fillStyle = fill; ctx.fill();
    trace();
    const stroke = ctx.createLinearGradient(plot.left, 0, plot.right, 0);
    stroke.addColorStop(0, '#f06a25'); stroke.addColorStop(.55, '#ffad82'); stroke.addColorStop(1, '#ffdbce');
    ctx.strokeStyle = stroke; ctx.lineWidth = 2.5; ctx.lineCap = 'round'; ctx.stroke();
  }
}

async function loadHistory(days) {
  const daily = days === 7 ? snapshot?.last_seven_days || [] : await invoke('usage_history', { days });
  if (days !== historyDays) return;
  historyPoints = daily || [];
  historyLoadedAt = Date.now();
  drawWeekChart(historyPoints);
}

function bindChartTooltip(canvasId, tooltipId, pointsFromSnapshot, labelForPoint) {
  const canvas = $(canvasId);
  const tooltip = $(tooltipId);
  canvas.addEventListener('mousemove', (event) => {
    const points = pointsFromSnapshot();
    if (!points.length || points.every((point) => point.tokens === 0)) { tooltip.style.display = 'none'; return; }
    const bounds = canvas.getBoundingClientRect();
    const x = event.clientX - bounds.left;
    const positions = canvas.chartPositions || [];
    const index = positions.length === points.length
      ? positions.reduce((best, position, candidate) => Math.abs(position - x) < Math.abs(positions[best] - x) ? candidate : best, 0)
      : Math.min(points.length - 1, Math.max(0, Math.floor(x / bounds.width * points.length)));
    const point = points[index];
    const label = document.createElement('strong'); label.textContent = labelForPoint(point);
    const value = document.createElement('span'); value.textContent = tr('overview.chartTokens', { tokens: formatTokens(point.tokens) });
    tooltip.replaceChildren(label, value);
    tooltip.style.display = 'block';
    tooltip.style.left = `${Math.max(0, Math.min(event.offsetX + 12, bounds.width - tooltip.offsetWidth))}px`;
    tooltip.style.top = `${Math.max(0, event.offsetY - tooltip.offsetHeight - 8)}px`;
  });
  canvas.addEventListener('mouseleave', () => { tooltip.style.display = 'none'; });
}

bindChartTooltip('chart-canvas', 'hour-tooltip', () => snapshot?.hourly || [], (point) => tr('overview.chartHour', { hour: String(point.hour).padStart(2, '0'), next: String((point.hour + 1) % 24).padStart(2, '0') }));
bindChartTooltip('week-chart-canvas', 'week-tooltip', () => historyPoints, (point) => {
  const format = (date) => new Intl.DateTimeFormat(document.documentElement.lang || undefined, { dateStyle: 'medium' }).format(new Date(`${date}T12:00:00`));
  return format(point.date);
});

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

async function exportUsage(format, selectedDays = Number($('export-range').value)) {
  const days = selectedDays;
  try {
    const exported = await invoke('export_usage_events', { days: days === 1 ? 7 : days });
    const events = days === 1 ? exported.filter((event) => new Date(event.timestamp).toDateString() === new Date().toDateString()) : exported;
    const disclaimer = 'Local observed usage; some counts are estimates. Not a bill. Times are UTC.';
    const fields = ['source', 'timestamp', 'tokens', 'input_tokens', 'output_tokens', 'cache_read', 'cache_write', 'is_estimated'];
    const quote = (value) => `"${String(value ?? '').replaceAll('"', '""')}"`;
    const content = format === 'csv'
      ? `# ${disclaimer}\r\n${fields.join(',')}\r\n${events.map((event) => fields.map((field) => quote(event[field])).join(',')).join('\r\n')}\r\n`
      : JSON.stringify({ disclaimer, range_days: days, events }, null, 2);
    const blob = new Blob([format === 'csv' ? '\ufeff' : '', content], { type: format === 'csv' ? 'text/csv;charset=utf-8' : 'application/json;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `tokenblaze-usage-${days}d-${new Date().toISOString().slice(0, 10)}.${format}`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  } catch { showError(); }
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
let refreshTimer;
let refreshInFlight = false;
async function refresh() {
  if (refreshInFlight) return;
  window.clearTimeout(refreshTimer);
  refreshInFlight = true;
  let nextRefreshDelay = refreshIntervalMs;
  try {
    updateSnapshot(await invoke('dashboard_snapshot'));
    if (historyDays !== 7 && Date.now() - historyLoadedAt > 60000) await loadHistory(historyDays);
    if (configValue(snapshot?.config || {}, 'costEnabled', 'cost_enabled') && Date.now() - costSummaryLoadedAt > 60000) await refreshCostSummary();
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
  refreshInFlight = false;
  refreshTimer = window.setTimeout(refresh, document.hidden ? 15000 : nextRefreshDelay);
}
document.addEventListener('visibilitychange', () => {
  if (!document.hidden && !refreshInFlight) {
    window.clearTimeout(refreshTimer);
    refresh();
  }
});
function run(command, args = {}) { return invoke(command, args).then(refresh).catch((error) => { console.error(`command failed: ${command}`, error); showError(); }); }

document.querySelectorAll('.nav-item').forEach((button) => button.addEventListener('click', () => {
  document.querySelectorAll('.nav-item').forEach((item) => {
    item.classList.toggle('active', item === button);
    if (item === button) item.setAttribute('aria-current', 'page'); else item.removeAttribute('aria-current');
  });
  document.querySelectorAll('.view').forEach((view) => view.classList.toggle('active', view.id === `view-${button.dataset.view}`));
  applyLanguage(snapshot?.config?.language || 'English');
  const heading = document.querySelector(`#view-${button.dataset.view} h1`);
  if (heading) { heading.tabIndex = -1; heading.focus(); }
}));
$('reduce-motion').addEventListener('change', (event) => run('set_reduce_motion', { enabled: event.target.checked }));
$('show-live-rate').addEventListener('change', (event) => run('set_show_live_rate', { enabled: event.target.checked }));
$('token-poll-interval').addEventListener('change', (event) => run('set_token_poll_interval', { seconds: Number(event.target.value) }));
$('panel-visible').addEventListener('change', (event) => run('set_panel_visible', { visible: event.target.checked }));
$('animation-paused').addEventListener('change', (event) => run('set_animation_paused', { paused: event.target.checked }));
$('flame-size').addEventListener('change', (event) => run('set_flame_size', { size: event.target.value }));
$('language').addEventListener('change', (event) => run('set_language', { language: event.target.value }));
$('theme').addEventListener('change', (event) => run('set_theme', { theme: event.target.value }));
$('source-path-source').addEventListener('change', () => refresh());
$('save-source-path').addEventListener('click', async () => {
  try {
    await invoke('set_source_path', { source: $('source-path-source').value, path: $('source-path-value').value });
    await refresh();
  } catch { showError(); }
});
$('clear-source-path').addEventListener('click', async () => {
  try {
    await invoke('set_source_path', { source: $('source-path-source').value, path: null });
    $('source-path-value').value = '';
    await refresh();
  } catch { showError(); }
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
const releasesButton = document.createElement('button');
releasesButton.id = 'open-releases';
releasesButton.className = 'text-button';
releasesButton.dataset.i18n = 'update.openReleases';
releasesButton.hidden = true;
document.querySelector('.update-actions').append(releasesButton);
releasesButton.addEventListener('click', () => invoke('open_releases').catch(showError));
const dataControls = document.createElement('div');
dataControls.className = 'data-controls';
dataControls.innerHTML = `
  <div class="setting-row"><div><strong data-i18n="settings.dataRetention"></strong><small data-i18n="settings.dataRetentionHint"></small></div>
    <select id="retention-days" aria-label="Data retention"><option value="0" data-i18n="settings.forever"></option><option value="30" data-i18n="settings.days" data-days="30"></option><option value="90" data-i18n="settings.days" data-days="90"></option><option value="365" data-i18n="settings.days" data-days="365"></option></select></div>
  <div class="setting-row"><div><strong data-i18n="settings.clearUsage"></strong><small data-i18n="settings.clearUsageHint"></small></div><button class="text-button danger-button" id="clear-usage" data-i18n="settings.clearUsage"></button></div>`;
document.querySelector('.update-card').before(dataControls);
const exportControls = document.createElement('div');
exportControls.className = 'setting-row export-controls';
exportControls.innerHTML = '<div><strong data-i18n="overview.exportData"></strong><small data-i18n="overview.exportHint"></small></div><div class="export-actions"><select id="export-range" aria-label="Export range"><option value="7" data-i18n="overview.sevenDays"></option><option value="30" data-i18n="overview.thirtyDays"></option><option value="90" data-i18n="overview.ninetyDays"></option></select><button class="text-button" id="export-csv" data-i18n="overview.exportCsv"></button><button class="text-button" id="export-json" data-i18n="overview.exportJson"></button></div>';
dataControls.before(exportControls);
$('export-csv').addEventListener('click', () => exportUsage('csv'));
$('export-json').addEventListener('click', () => exportUsage('json'));
const onboardingRow = document.createElement('div');
onboardingRow.className = 'setting-row';
onboardingRow.innerHTML = '<div><strong data-i18n="onboarding.title"></strong><small data-i18n="onboarding.step1Hint"></small></div><button class="text-button" id="onboarding-reopen" data-i18n="onboarding.reopen"></button>';
exportControls.before(onboardingRow);
const costCard = document.createElement('article');
costCard.className = 'card cost-card';
costCard.id = 'cost-card';
costCard.hidden = true;
costCard.innerHTML = '<p class="eyebrow" data-i18n="cost.title"></p><div class="cost-totals"><div><small id="cost-today-label"></small><strong id="cost-today">—</strong></div><div><small id="cost-week-label"></small><strong id="cost-week">—</strong></div></div><small id="cost-status" data-i18n="cost.hint"></small><p id="cost-budget-alert" data-i18n="cost.budgetAlert" hidden></p><button type="button" class="text-button cost-configure" data-i18n="cost.configure"></button>';
document.querySelector('.overview-grid').insertBefore(costCard, document.querySelector('.overview-grid .chart-card'));
costCard.querySelector('.cost-configure').addEventListener('click', () => document.querySelector('[data-view="settings"]').click());
document.querySelector('.metric-card').append(document.querySelector('.breakdown-card'));
const costSettings = document.createElement('div');
costSettings.className = 'cost-settings';
costSettings.innerHTML = `
  <div class="setting-row"><div><strong data-i18n="cost.title"></strong><small data-i18n="cost.hint"></small></div><label class="switch"><input id="cost-enabled" type="checkbox"><span></span></label></div>
  <details><summary data-i18n="cost.rates"></summary><p class="cost-rates-hint" data-i18n="cost.ratesHint"></p><div class="cost-table-wrap"><table class="cost-table"><thead><tr><th></th><th data-i18n="stats.input"></th><th data-i18n="stats.output"></th><th data-i18n="stats.cacheRead"></th><th data-i18n="stats.cacheWrite"></th></tr></thead><tbody id="cost-rates-body"></tbody></table></div></details>
  <div class="cost-budget-fields"><label><span data-i18n="cost.dailyBudget"></span><input id="cost-daily-budget" type="number" min="0.000001" max="1000000" step="any" inputmode="decimal"></label><label><span data-i18n="cost.weeklyBudget"></span><input id="cost-weekly-budget" type="number" min="0.000001" max="1000000" step="any" inputmode="decimal"></label></div>
  <button class="action-button compact" id="cost-save" data-i18n="cost.save"></button>`;
costSettings.querySelector('details').open = true;
onboardingRow.before(costSettings);
const appearancePanel = document.querySelector('#view-appearance .page-panel');
const appearanceSettings = appearancePanel.querySelector('.settings-card');
const appearanceLayout = document.createElement('div'); appearanceLayout.className = 'appearance-layout';
const previewPanel = document.createElement('section'); previewPanel.className = 'appearance-preview card';
previewPanel.innerHTML = '<div class="card-heading"><h2 data-i18n="appearance.previewCanvas"></h2><span class="phase-tag" data-i18n="appearance.floating"></span></div><div class="appearance-stage"><canvas id="appearance-flame-canvas" width="440" height="440"></canvas><div><strong id="preview-rate">—</strong><small id="preview-tier">—</small></div></div><div class="preview-status"><span data-i18n="overview.intensity"></span><strong id="preview-intensity">—</strong></div>';
const appearanceControls = document.createElement('div'); appearanceControls.className = 'appearance-controls';
const surfaceSection = document.createElement('section'); surfaceSection.className = 'appearance-section card';
surfaceSection.innerHTML = '<h2 data-i18n="appearance.surfaceSection"></h2><div class="theme-choices" role="group" aria-label="Theme"><button type="button" data-theme-choice="Dark"><span class="theme-swatches"><i></i><i></i><i></i></span><strong data-i18n="theme.dark"></strong></button><button type="button" data-theme-choice="Light"><span class="theme-swatches"><i></i><i></i><i></i></span><strong data-i18n="theme.light"></strong></button><button type="button" data-theme-choice="System"><span class="theme-swatches"><i></i><i></i><i></i></span><strong data-i18n="theme.system"></strong></button></div>';
surfaceSection.append(themeRow, $('reduce-motion').closest('.setting-row'));
const flameSection = document.createElement('section'); flameSection.className = 'appearance-section card';
flameSection.innerHTML = '<h2 data-i18n="appearance.flameSection"></h2>';
flameSection.append($('show-live-rate').closest('.setting-row'), $('animation-paused').closest('.setting-row'), $('flame-size').closest('.setting-row'), appearanceSettings.querySelector('.preview-tools'));
const paletteSection = document.createElement('section'); paletteSection.className = 'appearance-section card';
paletteSection.innerHTML = '<h2 data-i18n="appearance.paletteSection"></h2>';
paletteSection.append(appearanceSettings.querySelector('.palette'));
appearanceControls.append(surfaceSection, flameSection, paletteSection);
appearanceLayout.append(previewPanel, appearanceControls); appearancePanel.append(appearanceLayout);
appearanceSettings.remove();
const sourcesRescan = document.createElement('button');
sourcesRescan.type = 'button';
sourcesRescan.className = 'action-button sources-rescan';
sourcesRescan.dataset.i18n = 'settings.rescan';
document.querySelector('.sources-heading').append(sourcesRescan);
sourcesRescan.addEventListener('click', () => $('rescan').click());
const navSourceCount = document.createElement('span');
navSourceCount.id = 'nav-source-count';
navSourceCount.className = 'nav-source-count';
document.querySelector('[data-view="sources"]').append(navSourceCount);
document.querySelectorAll('[data-theme-choice]').forEach((button) => button.addEventListener('click', () => {
  $('theme').value = button.dataset.themeChoice;
  $('theme').dispatchEvent(new Event('change', { bubbles: true }));
}));

const settingsCard = document.querySelector('#view-settings .settings-card');
function settingsSection(key, className, ...items) {
  const section = document.createElement('section'); section.className = `settings-section card ${className}`;
  const title = document.createElement('h2'); title.dataset.i18n = key; section.append(title, ...items);
  settingsCard.append(section); return section;
}
settingsSection('settings.costSection', 'settings-cost', costSettings);
settingsSection('settings.dataSection', 'settings-data', dataControls);
const retentionChoices = document.createElement('div');
retentionChoices.className = 'retention-choices';
retentionChoices.setAttribute('role', 'group');
retentionChoices.setAttribute('aria-label', 'Data retention');
retentionChoices.innerHTML = '<button type="button" data-retention-choice="30" data-i18n="settings.days" data-days="30"></button><button type="button" data-retention-choice="90" data-i18n="settings.days" data-days="90"></button><button type="button" data-retention-choice="365" data-i18n="settings.days" data-days="365"></button><button type="button" data-retention-choice="0" data-i18n="settings.forever"></button>';
dataControls.querySelector('.setting-row').after(retentionChoices);
retentionChoices.querySelectorAll('button').forEach((button) => button.addEventListener('click', () => {
  $('retention-days').value = button.dataset.retentionChoice;
  $('retention-days').dispatchEvent(new Event('change', { bubbles: true }));
}));
settingsSection('settings.systemSection', 'settings-system', pollFrequencyRow, $('panel-visible').closest('.setting-row'), $('language').closest('.setting-row'), $('rescan'), settingsCard.querySelector('.source-path-settings'), settingsCard.querySelector('.debug-tools'), onboardingRow, exportControls);
settingsSection('settings.updateSection', 'settings-update', settingsCard.querySelector('.update-card'), settingsCard.querySelector('.privacy-note'));
const rateKinds = ['input', 'output', 'cache_read', 'cache_write'];
$('cost-rates-body').replaceChildren(...sourceNames.map((name, source) => {
  const row = document.createElement('tr');
  const heading = document.createElement('th');
  heading.scope = 'row'; heading.textContent = name; row.append(heading);
  rateKinds.forEach((kind) => {
    const cell = document.createElement('td');
    const input = document.createElement('input');
    input.type = 'number'; input.min = '0'; input.max = '1000000'; input.step = 'any'; input.inputMode = 'decimal';
    input.className = 'cost-rate'; input.dataset.source = String(source); input.dataset.kind = kind;
    input.setAttribute('aria-label', `${name} ${kind.replaceAll('_', ' ')} USD per million tokens`);
    cell.append(input); row.append(cell);
  });
  return row;
}));
const optionalPrice = (input) => {
  if (input.value.trim() === '') return null;
  const value = Number(input.value);
  if (!Number.isFinite(value) || value < 0 || value > 1000000) throw new Error('invalid price');
  return value;
};
const optionalBudget = (input) => {
  const value = optionalPrice(input);
  if (value !== null && value <= 0) throw new Error('invalid budget');
  return value;
};
$('cost-save').addEventListener('click', async () => {
  try {
    if ($('cost-enabled').checked && !configValue(snapshot?.config || {}, 'costEnabled', 'cost_enabled') && !window.confirm(tr('cost.confirmEnable'))) return;
    const rates = sourceNames.map(() => ({ input: null, output: null, cache_read: null, cache_write: null }));
    document.querySelectorAll('.cost-rate').forEach((input) => { rates[Number(input.dataset.source)][input.dataset.kind] = optionalPrice(input); });
    await invoke('set_cost_settings', {
      enabled: $('cost-enabled').checked, rates,
      dailyBudgetUsd: optionalBudget($('cost-daily-budget')),
      weeklyBudgetUsd: optionalBudget($('cost-weekly-budget')),
    });
    updateSnapshot(await invoke('dashboard_snapshot'));
    await refreshCostSummary();
  } catch { showError(); }
});
async function refreshCostSummary() {
  if (!snapshot || !configValue(snapshot.config, 'costEnabled', 'cost_enabled')) return;
  const summary = await invoke('cost_summary');
  if (!summary) return;
  const money = (amount) => new Intl.NumberFormat(document.documentElement.lang || undefined, { style: 'currency', currency: 'USD', minimumFractionDigits: 2, maximumFractionDigits: 4 }).format(amount);
  $('cost-today').textContent = money(summary.today.usd);
  $('topbar-cost').textContent = money(summary.today.usd);
  $('cost-week').textContent = money(summary.week.usd);
  $('cost-status').dataset.i18n = summary.today.incomplete || summary.week.incomplete ? 'cost.partial' : 'cost.hint';
  $('cost-status').textContent = tr($('cost-status').dataset.i18n);
  const overBudget = (summary.dailyBudgetUsd != null && summary.today.usd >= summary.dailyBudgetUsd)
    || (summary.weeklyBudgetUsd != null && summary.week.usd >= summary.weeklyBudgetUsd);
  $('cost-budget-alert').hidden = !overBudget;
  costCard.classList.toggle('over-budget', overBudget);
  costSummaryLoadedAt = Date.now();
}
const onboardingDialog = document.createElement('dialog');
onboardingDialog.className = 'onboarding-dialog';
onboardingDialog.innerHTML = `
  <p class="eyebrow">TOKENBLAZE</p><h2 data-i18n="onboarding.title"></h2>
  <section class="onboarding-step"><h3 data-i18n="onboarding.step1"></h3><p data-i18n="onboarding.step1Hint"></p><button class="action-button" id="onboarding-scan" data-i18n="onboarding.scan"></button></section>
  <section class="onboarding-step" hidden><h3 data-i18n="onboarding.step2"></h3><p data-i18n="onboarding.step2Hint"></p><strong id="onboarding-result"></strong></section>
  <section class="onboarding-step" hidden><h3 data-i18n="onboarding.step3"></h3><p data-i18n="onboarding.step3Hint"></p><label class="onboarding-choice"><input type="checkbox" id="onboarding-panel"><span data-i18n="settings.panel"></span></label></section>
  <div class="onboarding-actions"><button class="text-button" id="onboarding-skip" data-i18n="onboarding.skip"></button><button class="text-button" id="onboarding-back" data-i18n="onboarding.back"></button><button class="action-button" id="onboarding-next" data-i18n="onboarding.next"></button></div>`;
document.body.append(onboardingDialog);
function showOnboarding(step) {
  onboardingStep = step;
  onboardingDialog.querySelectorAll('.onboarding-step').forEach((section, index) => { section.hidden = index !== step; });
  $('onboarding-back').hidden = step === 0;
  $('onboarding-next').textContent = tr(step === 2 ? 'onboarding.finish' : 'onboarding.next');
  if (!onboardingDialog.open) onboardingDialog.showModal();
}
async function closeOnboarding(savePanel) {
  try {
    if (savePanel) await invoke('set_panel_visible', { visible: $('onboarding-panel').checked });
    await invoke('complete_onboarding');
    onboardingSeen = true;
    onboardingDialog.close();
  } catch { showError(); }
}
$('onboarding-reopen').addEventListener('click', () => {
  $('onboarding-panel').checked = Boolean(configValue(snapshot?.config || {}, 'panelVisible', 'panel_visible'));
  showOnboarding(0);
});
$('onboarding-scan').addEventListener('click', () => invoke('rescan').catch(showError));
$('onboarding-skip').addEventListener('click', () => closeOnboarding(false));
$('onboarding-back').addEventListener('click', () => showOnboarding(onboardingStep - 1));
$('onboarding-next').addEventListener('click', () => onboardingStep === 2 ? closeOnboarding(true) : showOnboarding(onboardingStep + 1));
onboardingDialog.addEventListener('cancel', (event) => { event.preventDefault(); closeOnboarding(false); });
const historyRange = document.createElement('select');
historyRange.id = 'history-range';
historyRange.innerHTML = '<option value="7" data-i18n="overview.sevenDays"></option><option value="30" data-i18n="overview.thirtyDays"></option><option value="90" data-i18n="overview.ninetyDays"></option>';
const historyHeading = $('week-chart-canvas').closest('.chart-card').querySelector('.card-heading');
historyHeading.querySelector('h2').id = 'history-title';
historyHeading.append(historyRange);
historyRange.addEventListener('change', async () => {
  const previous = historyDays;
  historyDays = Number(historyRange.value);
  $('history-title').textContent = tr({ 7: 'overview.sevenDays', 30: 'overview.thirtyDays', 90: 'overview.ninetyDays' }[historyDays]);
  $('week-chart-canvas').setAttribute('aria-label', $('history-title').textContent);
  try { await loadHistory(historyDays); }
  catch { historyDays = previous; historyRange.value = String(previous); showError(); }
});
const hourlyCard = $('chart-canvas').closest('.chart-card');
const historyCard = $('week-chart-canvas').closest('.chart-card');
[[hourlyCard, 'overview.hourlyHint'], [historyCard, 'overview.historyHint']].forEach(([card, key]) => {
  const hint = document.createElement('small');
  hint.className = 'chart-hint';
  hint.dataset.i18n = key;
  hint.textContent = tr(key);
  card.querySelector('.card-heading > div').append(hint);
});
const chartResizeObserver = new ResizeObserver(() => {
  if (snapshot) { drawChart(snapshot.hourly || []); drawWeekChart(historyPoints); }
});
chartResizeObserver.observe($('chart-canvas').parentElement);
chartResizeObserver.observe($('week-chart-canvas').parentElement);
historyCard.hidden = true;
document.querySelectorAll('[data-history-days]').forEach((button) => button.addEventListener('click', () => {
  overviewRangeDays = Number(button.dataset.historyDays);
  document.querySelectorAll('[data-history-days]').forEach((item) => item.classList.toggle('active', item === button));
  hourlyCard.hidden = overviewRangeDays !== 1;
  historyCard.hidden = overviewRangeDays === 1;
  if (overviewRangeDays !== 1) { historyRange.value = String(overviewRangeDays); historyRange.dispatchEvent(new Event('change')); }
}));
$('top-export-csv').addEventListener('click', () => exportUsage('csv', overviewRangeDays));
$('top-export-json').addEventListener('click', () => exportUsage('json', overviewRangeDays));
$('overview-rescan').addEventListener('click', () => $('rescan').click());
$('retention-days').addEventListener('change', async (event) => {
  const days = Number(event.target.value);
  if (days !== 0 && !window.confirm(tr('settings.confirmRetention'))) {
    event.target.value = String(configValue(snapshot?.config || {}, 'retentionDays', 'retention_days') ?? 0);
    return;
  }
  try { await invoke('set_retention_days', { days }); updateSnapshot(await invoke('dashboard_snapshot')); }
  catch { showError(); event.target.value = String(configValue(snapshot?.config || {}, 'retentionDays', 'retention_days') ?? 0); }
});
$('clear-usage').addEventListener('click', async () => {
  if (!window.confirm(tr('settings.confirmClear'))) return;
  try { await invoke('clear_usage_data'); updateSnapshot(await invoke('dashboard_snapshot')); }
  catch { showError(); }
});
overviewFlame = window.HDFireRenderer.attach($('flame-canvas'));
appearanceFlame = window.HDFireRenderer.attach($('appearance-flame-canvas'));
refresh();
