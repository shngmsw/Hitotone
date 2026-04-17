// Hitotone Renderer Process - Main Entry Point (Tauri v2)

import { invoke } from '@tauri-apps/api/core';
import { AiCompanionManager } from './AiCompanionManager.js';
import { EventManager } from './EventManager.js';
import { LoadingManager } from './LoadingManager.js';
import { ServiceDockManager } from './ServiceDockManager.js';
import { SettingsManager } from './SettingsManager.js';
import { UpdateManager } from './UpdateManager.js';
import { WebViewManager } from './WebViewManager.js';

class Hitotone {
  constructor() {
    // State
    this.services = [];
    this.activeServiceId = null;
    this.aiServices = [];
    this.activeAiService = null;

    // Managers
    this.loadingManager = new LoadingManager();
    this.serviceDockManager = new ServiceDockManager(this);
    this.webViewManager = new WebViewManager(this);
    this.aiCompanionManager = new AiCompanionManager(this);
    this.settingsManager = new SettingsManager(this);
    this.eventManager = new EventManager(this);
    this.updateManager = new UpdateManager();

    this.init();
  }

  async init() {
    try {
      console.log('[Hitotone] init() start');

      // プラットフォーム判定してbodyにクラスを追加
      const platform = await invoke('get_platform');
      document.body.classList.add(`platform-${platform}`);
      console.log('[Hitotone] platform:', platform);

      // 設定を読み込み
      this.services = await invoke('get_services');
      console.log('[Hitotone] services:', this.services);
      this.activeServiceId = await invoke('get_active_service');
      console.log('[Hitotone] activeServiceId:', this.activeServiceId);
      this.aiServices = await invoke('get_ai_services');
      console.log('[Hitotone] aiServices:', this.aiServices);
      this.activeAiService = await invoke('get_active_ai_service');
      console.log('[Hitotone] activeAiService:', this.activeAiService);
      const showAiCompanion = await invoke('get_show_ai_companion');
      const aiWidth = await invoke('get_ai_width');
      console.log('[Hitotone] showAiCompanion:', showAiCompanion, 'aiWidth:', aiWidth);

      // サービスドックを構築
      this.serviceDockManager.render();
      console.log('[Hitotone] dock rendered');

      // イベントリスナーを設定（先に登録してボタンを確実に動くようにする）
      this.eventManager.setup();
      console.log('[Hitotone] event listeners setup done');

      // リサイズハンドルを設定
      this.aiCompanionManager.setupResizeHandle();

      // WebViewをRust側で作成
      try {
        console.log('[Hitotone] creating webviews...');
        await this.webViewManager.createWebViews();
        console.log('[Hitotone] webviews created');
      } catch (err) {
        console.warn('[Hitotone] webview creation error (non-fatal):', err);
      }

      // AIコンパニオンを設定
      try {
        console.log('[Hitotone] setting up AI companion...');
        await this.aiCompanionManager.setup(showAiCompanion, aiWidth);
        console.log('[Hitotone] AI companion setup done');
      } catch (err) {
        console.warn('[Hitotone] AI companion setup error (non-fatal):', err);
      }

      // 初期サービスをアクティブに
      // 初期サービスをアクティブに
      if (this.services.length === 0) {
        console.log('[Hitotone] No services found, showing onboarding');
        const onboarding = document.getElementById('onboarding-screen');
        if (onboarding) this.showModal(onboarding);
      } else if (this.activeServiceId) {
        // ... (check validity of activeServiceId?)
        // wait, earlier if activeServiceId doesn't exist in services, fallback to services[0].
        const hasActiveService = this.services.find((s) => s.id === this.activeServiceId);
        if (hasActiveService) {
          console.log('[Hitotone] switching to active service:', this.activeServiceId);
          await this.webViewManager.switchService(this.activeServiceId);
        } else {
          console.log('[Hitotone] switching to first service:', this.services[0].id);
          await this.webViewManager.switchService(this.services[0].id);
        }
      } else if (this.services.length > 0) {
        console.log('[Hitotone] switching to first service:', this.services[0].id);
        await this.webViewManager.switchService(this.services[0].id);
      }

      // ローディング非表示
      this.loadingManager.hide();
      console.log('[Hitotone] init() complete - loading hidden');
    } catch (error) {
      console.error('Hitotone initialization failed:', error);
      console.error('Error stack:', error.stack || error);
    }
  }

  async showModal(modalElement) {
    if (!modalElement) return;
    await invoke('hide_all_child_webviews');
    modalElement.classList.remove('hidden');
  }

  async hideModal(modalElement) {
    if (!modalElement) return;
    modalElement.classList.add('hidden');

    // 他のモーダルが開いていないか確認
    const anyOpen = Array.from(document.querySelectorAll('.modal, #onboarding-screen')).some(
      (m) => !m.classList.contains('hidden'),
    );
    if (!anyOpen) {
      await invoke('restore_child_webviews');
    }
  }
}

// アプリ起動
document.addEventListener('DOMContentLoaded', () => {
  new Hitotone();
});
