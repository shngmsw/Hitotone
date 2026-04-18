// Hitotone Renderer Process - Main Entry Point (Tauri v2)

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { AiCompanionManager } from './AiCompanionManager.js';
import { EventManager } from './EventManager.js';
import { LoadingManager } from './LoadingManager.js';
import { PaneTreeManager } from './PaneTreeManager.js';
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
    this.webViewManager = new WebViewManager(this);
    this.aiCompanionManager = new AiCompanionManager(this);
    this.settingsManager = new SettingsManager(this);
    this.eventManager = new EventManager(this);
    this.updateManager = new UpdateManager();
    this.paneTreeManager = new PaneTreeManager(this);

    this.init();
  }

  async init() {
    try {
      console.log('[Hitotone] init() start');

      // タイトルバーボタン
      const win = getCurrentWindow();
      document.getElementById('tb-minimize')?.addEventListener('click', () => win.minimize());
      document.getElementById('tb-maximize')?.addEventListener('click', () => win.toggleMaximize());
      document.getElementById('tb-close')?.addEventListener('click', () => win.close());

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

      // イベントリスナーを設定（先に登録してボタンを確実に動くようにする）
      this.eventManager.setup();
      console.log('[Hitotone] event listeners setup done');

      // リサイズハンドルを設定
      this.aiCompanionManager.setupResizeHandle();

      // ウィンドウリサイズ時のレイアウト更新（RustのネイティブスレッドではなくJSから非同期で呼ぶ）
      let resizeTimeout;
      window.addEventListener('resize', () => {
        clearTimeout(resizeTimeout);
        resizeTimeout = setTimeout(() => {
          invoke('update_layout').catch(console.error);
        }, 50);
      });

      // WebViewをRust側で作成
      try {
        console.log('[Hitotone] creating webviews...');
        await this.webViewManager.createWebViews();
        console.log('[Hitotone] webviews created');
      } catch (err) {
        console.warn('[Hitotone] webview creation error (non-fatal):', err);
      }

      // ペインツリーDOM初期化
      await this.paneTreeManager.init();

      // pane-tree-updated イベントでDOM再描画（dock クリック / split / close 後）
      listen('pane-tree-updated', () => this.paneTreeManager.refresh());

      // AIコンパニオンを設定
      try {
        console.log('[Hitotone] setting up AI companion...');
        await this.aiCompanionManager.setup(showAiCompanion, aiWidth);
        console.log('[Hitotone] AI companion setup done');
      } catch (err) {
        console.warn('[Hitotone] AI companion setup error (non-fatal):', err);
      }

      // サービス未設定ならオンボーディング
      if (this.services.length === 0) {
        console.log('[Hitotone] No services found, showing onboarding');
        const onboarding = document.getElementById('onboarding-screen');
        if (onboarding) this.showModal(onboarding);
      }

      // chrome WebView からのモーダル開要求をリッスン
      listen('open-modal', (e) => {
        const modalType = e.payload;
        if (modalType === 'add-service') {
          const modal = document.getElementById('add-service-modal');
          if (modal) this.showModal(modal);
        } else if (modalType === 'settings') {
          this.settingsManager.open();
        }
      });

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
