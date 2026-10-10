const request = new URLSearchParams(location.search).get('request');
const element = (id) => document.getElementById(id);
const form = element('consent');
const openApp = element('open-app');
const returnLink = element('return');
const messages = {
  pending: '请在运行此连接的设备上确认授权。',
  approved: '已允许连接，正在返回客户端…',
  denied: '已拒绝连接，正在返回客户端…',
  completed: '授权已完成，可以关闭此页面。',
  expired: '请求已过期，请从客户端重新连接。',
};
let redirecting = false;
let timer;
function render(state) {
  element('status').textContent = messages[state.status] || messages.expired;
  element('title').textContent = state.status === 'completed' ? '已连接' : '连接授权';
  form.hidden = state.status !== 'pending';
  if (state.openAppUrl) openApp.href = state.openAppUrl;
  element('details').hidden = !state.clientName;
  element('client').textContent = state.clientName || '';
  element('connection').textContent = state.connectionName || state.resource || '';
  try { element('callback').textContent = new URL(state.redirectUri).hostname; }
  catch { element('callback').textContent = '—'; }
  if (['approved', 'denied'].includes(state.status)) {
    returnLink.href = `/oauth/continue?request=${encodeURIComponent(request)}`;
    returnLink.hidden = false;
    if (!redirecting) {
      redirecting = true;
      setTimeout(() => location.assign(returnLink.href), 700);
    }
  }
  if (['completed', 'expired'].includes(state.status)) clearTimeout(timer);
}
async function refresh() {
  if (redirecting) return;
  try {
    const response = await fetch(`/oauth/status?request=${encodeURIComponent(request || '')}`, {cache:'no-store'});
    if (!response.ok) throw new Error('读取失败');
    const state = await response.json();
    render(state);
    if (state.status === 'pending') timer = setTimeout(refresh, 2500);
  } catch {
    element('status').textContent = '暂时无法读取状态，正在重试…';
    timer = setTimeout(refresh, 5000);
  }
}
if (request) refresh();
else render({status:'expired'});
