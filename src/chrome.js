import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { PresetPickerManager } from './PresetPickerManager.js';

let spaces = [];
let activeSpaceId = null;

async function init() {
  try {
    [spaces, activeSpaceId] = await Promise.all([
      invoke('get_spaces'),
      invoke('get_active_space_id'),
    ]);
  } catch (e) {
    console.error('[chrome] init error:', e);
  }
  renderSpaces();
  setupButtons();
  setupListeners();

  const presetPicker = new PresetPickerManager();
  presetPicker.init('preset-picker');
}

function renderSpaces() {
  const list = document.getElementById('space-list');
  if (!list) return;
  list.innerHTML = '';

  spaces.forEach((space, idx) => {
    const item = document.createElement('div');
    item.className = 'space-item' + (space.id === activeSpaceId ? ' active' : '');
    item.dataset.spaceId = space.id;
    item.title = space.name;
    item.textContent = String(idx + 1);

    const delBtn = document.createElement('button');
    delBtn.className = 'space-delete';
    delBtn.title = '削除';
    delBtn.textContent = '×';
    delBtn.addEventListener('click', async e => {
      e.stopPropagation();
      try {
        spaces = await invoke('delete_space', { spaceId: space.id });
        activeSpaceId = await invoke('get_active_space_id');
        renderSpaces();
      } catch (err) {
        console.warn('[chrome] delete_space:', err);
      }
    });
    item.appendChild(delBtn);

    item.addEventListener('click', () => switchSpace(space.id));
    list.appendChild(item);
  });
}

async function switchSpace(spaceId) {
  if (spaceId === activeSpaceId) return;
  try {
    await invoke('switch_space', { spaceId });
    activeSpaceId = spaceId;
    renderSpaces();
  } catch (e) {
    console.error('[chrome] switch_space error:', e);
  }
}

function setupButtons() {
  document.getElementById('add-space-btn')?.addEventListener('click', async () => {
    try {
      spaces = await invoke('create_space', { name: '' });
      renderSpaces();
    } catch (e) {
      console.error('[chrome] create_space error:', e);
    }
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
  listen('space-list-updated', async () => {
    spaces = await invoke('get_spaces');
    activeSpaceId = await invoke('get_active_space_id');
    renderSpaces();
  });

  listen('pane-tree-updated', async () => {
    activeSpaceId = await invoke('get_active_space_id');
    renderSpaces();
  });
}

document.addEventListener('DOMContentLoaded', init);
