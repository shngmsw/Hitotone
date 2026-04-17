// UpdateManager - 自動更新の通知UI管理 (Tauri v2)

import { check } from '@tauri-apps/plugin-updater';

export class UpdateManager {
  constructor() {
    this.banner = document.getElementById('update-banner');
    this.message = document.getElementById('update-message');
    this.progress = document.getElementById('update-progress');
    this.progressBar = document.getElementById('update-progress-bar');
    this.actions = document.getElementById('update-actions');
    this.update = null;

    this.setup();
  }

  async setup() {
    // 30秒後に初回チェック
    setTimeout(() => this.checkForUpdates(), 30000);

    // 6時間ごとにチェック
    setInterval(() => this.checkForUpdates(), 6 * 60 * 60 * 1000);
  }

  async checkForUpdates() {
    try {
      const update = await check();
      if (update) {
        this.update = update;
        this.showUpdateAvailable(update);
      }
    } catch (error) {
      console.warn('Update check failed:', error);
    }
  }

  showUpdateAvailable(update) {
    this.message.textContent = `新しいバージョン v${update.version} が利用可能です`;
    this.progress.classList.add('hidden');
    this.actions.innerHTML = `
      <button class="update-btn-secondary" id="update-later-btn">後で</button>
      <button class="update-btn-primary" id="update-download-btn">ダウンロード</button>
    `;

    document.getElementById('update-later-btn').onclick = () => this.hide();
    document.getElementById('update-download-btn').onclick = () => this.startDownload();

    this.show();
  }

  async startDownload() {
    if (!this.update) return;

    try {
      this.message.textContent = 'ダウンロード中...';
      this.progress.classList.remove('hidden');
      this.actions.innerHTML = '';
      this.show();

      await this.update.downloadAndInstall((event) => {
        if (event.event === 'Progress') {
          const percent = Math.round((event.data.chunkLength / event.data.contentLength) * 100);
          this.message.textContent = `ダウンロード中... ${percent}%`;
          this.progressBar.style.width = `${percent}%`;
        } else if (event.event === 'Finished') {
          this.showUpdateReady();
        }
      });
    } catch (error) {
      this.showError(error.toString());
    }
  }

  showUpdateReady() {
    this.message.textContent = 'アップデートの準備ができました';
    this.progress.classList.add('hidden');
    this.actions.innerHTML = `
      <button class="update-btn-secondary" id="update-later-btn">後で再起動</button>
      <button class="update-btn-primary" id="update-install-btn">今すぐ再起動</button>
    `;

    document.getElementById('update-later-btn').onclick = () => this.hide();
    document.getElementById('update-install-btn').onclick = () => {
      // Tauri plugin-updater: app restarts automatically after downloadAndInstall
      // If not, exit and let the user reopen
      import('@tauri-apps/api/core').then(({ invoke }) => invoke('window_close'));
    };

    this.show();
  }

  showError(message) {
    this.message.textContent = `更新エラー: ${message}`;
    this.progress.classList.add('hidden');
    this.actions.innerHTML = `
      <button class="update-btn-secondary" id="update-close-btn">閉じる</button>
    `;

    document.getElementById('update-close-btn').onclick = () => this.hide();

    this.show();

    // 5秒後に自動で閉じる
    setTimeout(() => this.hide(), 5000);
  }

  show() {
    this.banner.classList.remove('hidden');
  }

  hide() {
    this.banner.classList.add('hidden');
  }
}
