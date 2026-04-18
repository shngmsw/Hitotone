// SettingsManager.js - 設定モーダル管理 (Tauri v2)

import { invoke } from '@tauri-apps/api/core';

export class SettingsManager {
  constructor(hitotone) {
    this.hitotone = hitotone;
    this.initTheme();
    this.setupThemeListener();
  }

  // テーマの初期化
  initTheme() {
    const savedTheme = localStorage.getItem('hitotone-theme') || 'default';
    document.body.dataset.theme = savedTheme;

    // セレクターの初期値を設定
    const selector = document.getElementById('theme-selector');
    if (selector) {
      selector.value = savedTheme;
    }
  }

  // テーマ変更のリスナー設定
  setupThemeListener() {
    const selector = document.getElementById('theme-selector');
    if (selector) {
      selector.addEventListener('change', (e) => {
        const theme = e.target.value;
        document.body.dataset.theme = theme;
        localStorage.setItem('hitotone-theme', theme);
      });
    }
  }

  // XSS対策: HTMLエスケープ
  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  open() {
    // リストの描画前にテーマセレクターの状態を同期
    const savedTheme = localStorage.getItem('hitotone-theme') || 'default';
    const selector = document.getElementById('theme-selector');
    if (selector) {
      selector.value = savedTheme;
    }

    const list = document.getElementById('service-settings-list');
    if (!list) return;

    list.innerHTML = '';

    this.hitotone.services.forEach((service) => {
      const item = document.createElement('div');
      item.className = 'service-setting-item';
      item.innerHTML = `
        <div class="service-info">
          <span class="service-icon">${service.icon}</span>
          <div>
            <div class="service-name">${this.escapeHtml(service.name)}</div>
            <div class="service-url">${this.escapeHtml(service.url)}</div>
          </div>
        </div>
        <div class="service-actions">
          <button class="edit-btn" data-service-id="${this.escapeHtml(service.id)}" title="編集">✏️</button>
          <button class="delete-btn" data-service-id="${this.escapeHtml(service.id)}" title="削除">🗑️</button>
        </div>
      `;
      list.appendChild(item);
    });

    // 編集ボタンのイベント
    list.querySelectorAll('.edit-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        const serviceId = btn.dataset.serviceId;
        this.openEditService(serviceId);
      });
    });

    // 削除ボタンのイベント
    list.querySelectorAll('.delete-btn').forEach((btn) => {
      btn.addEventListener('click', async () => {
        const serviceId = btn.dataset.serviceId;
        if (confirm('このサービスを削除しますか？')) {
          this.hitotone.services = await invoke('remove_service', { serviceId });
          // serviceDockManager.render() 削除 (pane-tree-updated イベントで PaneTreeManager が自動更新)

          await invoke('remove_service_webview', { serviceId });

          if (this.hitotone.services.length === 0) {
            this.hitotone.activeServiceId = null;
            const onboarding = document.getElementById('onboarding-screen');
            if (onboarding) this.hitotone.showModal(onboarding);
          }

          await this.hitotone.paneTreeManager.refresh();
          this.open(); // リストを更新
        }
      });
    });

    const modal = document.getElementById('settings-modal');
    if (modal) {
      this.hitotone.showModal(modal);
    }
  }

  openEditService(serviceId) {
    const service = this.hitotone.services.find((s) => s.id === serviceId);
    if (!service) return;

    const idInput = document.getElementById('edit-service-id');
    const nameInput = document.getElementById('edit-service-name');
    const urlInput = document.getElementById('edit-service-url');
    const iconInput = document.getElementById('edit-service-icon');

    if (idInput) idInput.value = service.id;
    if (nameInput) nameInput.value = service.name;
    if (urlInput) urlInput.value = service.url;
    if (iconInput) iconInput.value = service.icon;

    const modal = document.getElementById('edit-service-modal');
    if (modal) {
      this.hitotone.showModal(modal);
    }
  }

  close() {
    const modal = document.getElementById('settings-modal');
    if (modal) {
      this.hitotone.hideModal(modal);
    }
  }
}
