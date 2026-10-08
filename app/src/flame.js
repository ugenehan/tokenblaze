const invoke = window.__TAURI__?.core?.invoke;
const panel = document.getElementById('panel');
const flameZone = document.getElementById('flame-zone');
const renderer = window.HDFireRenderer.attach(document.getElementById('fire'));
const systemTheme = window.matchMedia('(prefers-color-scheme: dark)');
let selectedTheme = 'Dark';
function applyTheme() {
  document.documentElement.dataset.theme = selectedTheme === 'System'
    ? (systemTheme.matches ? 'dark' : 'light') : selectedTheme.toLowerCase();
}
systemTheme.addEventListener('change', applyTheme);
const labels = {
  English: { today: 'TODAY', minute: '/ min', footer: 'Updated just now · Usage stays on this device', budget: 'Estimated budget reached · Not a bill', empty: 'No token usage today' },
  Chinese: { today: '今日燃烧', minute: '/ 分钟', footer: '刚刚更新 · 数据只保存在本机', budget: '估算成本已达预算 · 非账单', empty: '今日暂无 Token 消耗' },
  Japanese: { today: '本日の使用量', minute: '/ 分', footer: '更新済み · データはこの端末内に保存', budget: '推定予算に到達 · 請求額ではありません', empty: '本日のトークン使用はありません' },
  Korean: { today: '오늘 사용량', minute: '/ 분', footer: '방금 업데이트 · 데이터는 이 기기에만 저장', budget: '예상 예산 도달 · 청구 금액 아님', empty: '오늘 토큰 사용량이 없습니다' },
};
let busy = false;
let budgetExceeded = false;
let lastBudgetCheck = 0;
let requestedExpanded = false;
let windowExpanded = false;
let hoverTimer;
let hoverTransition = Promise.resolve();

const compact = (value) => new Intl.NumberFormat('en-US', {
  notation: 'compact', maximumFractionDigits: 1,
}).format(Math.max(0, value || 0));

function renderSummary(state) {
  selectedTheme = state.theme || 'Dark';
  applyTheme();
  const language = state.language === 'System'
    ? ({ zh: 'Chinese', ja: 'Japanese', ko: 'Korean' }[navigator.language.slice(0, 2).toLowerCase()] || 'English')
    : state.language;
  const copy = labels[language] || labels.English;
  document.documentElement.lang = { Chinese: 'zh-CN', Japanese: 'ja', Korean: 'ko' }[language] || 'en';
  const sizeScale = state.config?.flameSize === 'Small' ? 1
    : state.config?.flameSize === 'Large' ? 2.75 : 1.75;
  flameZone.style.setProperty('--flame-width', `${56 * sizeScale + 28}px`);
  flameZone.style.setProperty('--flame-height', `${96 * sizeScale + 36}px`);
  document.getElementById('today-label').textContent = copy.today;
  document.getElementById('daily-total').textContent = compact(state.todayTokens);
  document.getElementById('card-footer').textContent = budgetExceeded ? copy.budget : copy.footer;

  const rate = state.showLiveRate ? Number(state.fire?.tokens_per_second || 0) * 60 : 0;
  document.getElementById('live-rate').textContent = rate > 0
    ? `↑ ${compact(rate)} ${copy.minute}`
    : '';

  const colors = state.config?.sourceColors || [];
  const rows = (state.sources || [])
    .filter((source) => source.tokens > 0)
    .sort((left, right) => right.tokens - left.tokens)
    .slice(0, 3);
  const list = document.getElementById('source-list');
  list.replaceChildren();
  if (!rows.length) {
    const row = document.createElement('li');
    row.className = 'source-row empty-row';
    row.style.gridTemplateColumns = '1fr';
    row.textContent = copy.empty;
    list.append(row);
    return;
  }

  rows.forEach((source) => {
    const sourceIndex = (state.sources || []).findIndex((item) => item.id === source.id);
    const rgb = colors[sourceIndex] || [244, 104, 52];
    const row = document.createElement('li');
    row.className = 'source-row';
    const dot = document.createElement('span');
    dot.className = 'source-dot';
    dot.style.setProperty('--source-color', `rgb(${rgb.join(',')})`);
    dot.style.backgroundColor = `rgb(${rgb.join(',')})`;
    const name = document.createElement('span');
    name.className = 'source-name';
    name.textContent = source.name;
    const total = document.createElement('span');
    total.className = 'source-tokens';
    total.textContent = compact(source.tokens);
    row.append(dot, name, total);
    list.append(row);
  });
}

function setExpanded(next) {
  if (requestedExpanded === next) return;
  requestedExpanded = next;
  hoverTransition = hoverTransition.then(async () => {
    if (requestedExpanded !== next) return;
    if (next) {
      if (!windowExpanded && invoke) {
        await invoke('set_flame_hover', { expanded: true });
        windowExpanded = true;
      }
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      if (requestedExpanded) panel.classList.add('expanded');
    } else {
      panel.classList.remove('expanded');
      const fadeTime = window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 220;
      await new Promise((resolve) => window.setTimeout(resolve, fadeTime));
      if (!requestedExpanded && windowExpanded && invoke) {
        await invoke('set_flame_hover', { expanded: false });
        windowExpanded = false;
      }
    }
  }).catch((error) => {
    console.error('flame hover resize failed', error);
    requestedExpanded = false;
    panel.classList.remove('expanded');
  });
}

function scheduleExpanded(next) {
  window.clearTimeout(hoverTimer);
  hoverTimer = window.setTimeout(() => setExpanded(next), next ? 60 : 100);
}

async function updateFireState() {
  if (!busy && invoke) {
    busy = true;
    try {
      const state = await invoke('fire_visual_state');
      renderer.setState(state);
      renderSummary(state);
      if (Date.now() - lastBudgetCheck >= 60000) {
        const summary = await invoke('cost_summary');
        lastBudgetCheck = Date.now();
        budgetExceeded = Boolean(summary && ((summary.dailyBudgetUsd != null && summary.today.usd >= summary.dailyBudgetUsd)
          || (summary.weeklyBudgetUsd != null && summary.week.usd >= summary.weeklyBudgetUsd)));
        panel.classList.toggle('over-budget', budgetExceeded);
        document.getElementById('card-footer').textContent = budgetExceeded
          ? (labels[state.language] || labels.English).budget : (labels[state.language] || labels.English).footer;
      }
    } catch (error) {
      console.error('flame state failed', error);
    } finally {
      busy = false;
    }
  }
  window.setTimeout(updateFireState, 1000);
}

flameZone.addEventListener('mouseenter', () => scheduleExpanded(true));
panel.addEventListener('mouseenter', () => {
  window.clearTimeout(hoverTimer);
  if (windowExpanded) setExpanded(true);
});
panel.addEventListener('mouseleave', () => scheduleExpanded(false));
flameZone.addEventListener('mousedown', (event) => {
  if (event.button !== 0 || !invoke) return;
  invoke('start_dragging').catch((error) => {
    console.error('flame drag failed', error);
  });
});
panel.addEventListener('contextmenu', (event) => {
  event.preventDefault();
  invoke?.('open_console');
});
async function initializeFlameWindow() {
  if (invoke) {
    try {
      await invoke('initialize_flame_window');
    } catch (error) {
      console.error('flame window initialization failed', error);
    }
  }
  updateFireState();
}

initializeFlameWindow();
