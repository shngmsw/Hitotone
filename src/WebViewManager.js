// WebViewManager.js - WebView管理 (Tauri v2)
// WebView作成・切り替えはRust側に委譲

import { invoke } from '@tauri-apps/api/core';

export class WebViewManager {
  constructor(hitotone) {
    this.hitotone = hitotone;
    this.initialLoadDone = new Set();
    this.faviconExtracted = new Set();
  }

  async createWebViews() {
    // Rust側で全サービスのWebViewを作成
    await invoke('create_all_service_webviews');
  }

  async switchService(serviceId) {
    const service = this.hitotone.services.find((s) => s.id === serviceId);
    if (!service) return;

    // 前のアクティブを非アクティブに
    document.querySelectorAll('.service-item').forEach((item) => {
      item.classList.remove('active');
    });

    // Rust側でWebView切り替え
    await invoke('switch_service_webview', { serviceId });

    // アクティブなサービスを更新
    this.hitotone.activeServiceId = serviceId;
    const activeItem = document.querySelector(`.service-item[data-service-id="${serviceId}"]`);
    if (activeItem) {
      activeItem.classList.add('active');
    }

    // 設定を保存
    await invoke('set_active_service', { serviceId });
  }

  async reloadActiveWebview() {
    // Rust側にリロード要求（将来的な拡張）
    // 現時点ではWebView自体のリロードはRust側で対応
  }

  async navigateActiveWebview(url) {
    // Slackの場合、認証後のURLを保存する
    if (this.hitotone.activeServiceId === 'slack' && url && url.includes('app.slack.com/client/')) {
      const services = await invoke('update_service', {
        service: {
          ...this.hitotone.services.find((s) => s.id === 'slack'),
          url: url,
        },
      });
      this.hitotone.services = services;
    }
  }

  getWebview(_serviceId) {
    // WebView DOMは存在しないため、nullを返す
    return null;
  }
}
