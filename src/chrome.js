import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

let services = [];
let activeServiceId = null;

async function init() {
  try {
    [services, activeServiceId] = await Promise.all([
      invoke('get_services'),
      invoke('get_active_service'),
    ]);
  } catch (e) {
    console.error('[chrome] init error:', e);
  }
  renderDock();
  setupButtons();
  setupListeners();
}

function renderDock() {
  const list = document.getElementById('service-list');
  if (!list) return;
  list.innerHTML = '';
  services.filter(s => s.enabled).forEach(service => {
    const item = document.createElement('div');
    item.className = 'service-item' + (service.id === activeServiceId ? ' active' : '');
    item.dataset.serviceId = service.id;
    item.title = service.name;

    const faviconUrl = service.faviconUrl || service.favicon_url;
    if (faviconUrl) {
      const img = document.createElement('img');
      img.className = 'service-favicon';
      img.src = faviconUrl;
      img.alt = service.name;
      img.onerror = () => { img.replaceWith(document.createTextNode(service.icon)); };
      item.appendChild(img);
    } else {
      item.appendChild(document.createTextNode(service.icon));
    }

    const badge = document.createElement('span');
    badge.className = 'badge hidden';
    item.appendChild(badge);

    item.addEventListener('click', () => switchService(service.id));
    list.appendChild(item);
  });
}

async function switchService(serviceId) {
  await invoke('switch_service_webview', { serviceId });
  await invoke('set_active_service', { serviceId });
  activeServiceId = serviceId;
  document.querySelectorAll('.service-item').forEach(el => {
    el.classList.toggle('active', el.dataset.serviceId === serviceId);
  });
}


function setupButtons() {
  document.getElementById('add-service-btn')?.addEventListener('click', () => {
    invoke('request_open_modal', { modalType: 'add-service' }).catch(console.error);
  });

  document.getElementById('toggle-ai-btn')?.addEventListener('click', async () => {
    try {
      const show = await invoke('toggle_ai_webview');
      if (show) {
        const aiServices = await invoke('get_ai_services');
        const activeAi = await invoke('get_active_ai_service');
        const svc = aiServices.find(s => s.id === activeAi?.id) || aiServices[0];
        if (svc) await invoke('create_ai_webview', { url: svc.url }).catch(() => {});
      }
    } catch (e) { console.error('[chrome] ai toggle error:', e); }
  });

  document.getElementById('settings-btn')?.addEventListener('click', () => {
    invoke('request_open_modal', { modalType: 'settings' }).catch(console.error);
  });
}

function setupListeners() {
  listen('badge-updated', (e) => {
    const { serviceId, count } = e.payload;
    const item = document.querySelector(`.service-item[data-service-id="${serviceId}"]`);
    if (!item) return;
    const badge = item.querySelector('.badge');
    if (!badge) return;
    if (count > 0) {
      badge.textContent = count > 99 ? '99+' : String(count);
      badge.classList.remove('hidden');
    } else {
      badge.classList.add('hidden');
    }
  });

  listen('favicon-updated', (e) => {
    const { serviceId, faviconUrl } = e.payload;
    const item = document.querySelector(`.service-item[data-service-id="${serviceId}"]`);
    if (!item) return;
    const badge = item.querySelector('.badge');
    Array.from(item.childNodes).forEach(n => { if (n !== badge) n.remove(); });
    const img = document.createElement('img');
    img.className = 'service-favicon';
    img.src = faviconUrl;
    const svc = services.find(s => s.id === serviceId);
    img.alt = svc?.name || '';
    img.onerror = () => { img.replaceWith(document.createTextNode(svc?.icon || '🔗')); };
    item.insertBefore(img, badge);
  });

  listen('service-list-updated', async () => {
    services = await invoke('get_services');
    renderDock();
  });

  listen('active-service-changed', (e) => {
    activeServiceId = e.payload;
    document.querySelectorAll('.service-item').forEach(el => {
      el.classList.toggle('active', el.dataset.serviceId === activeServiceId);
    });
  });
}

document.addEventListener('DOMContentLoaded', init);
