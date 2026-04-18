// EventManager.js - イベントリスナー管理 (Tauri v2)

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export class EventManager {
  constructor(hitotone) {
    this.hitotone = hitotone;
  }

  setup() {
    this.setupAiSelectorEvents();
    this.setupServiceEvents();
    this.setupModalEvents();
    this.setupKeyboardShortcuts();
    this.setupTauriListeners();
  }

  setupAiSelectorEvents() {
    const aiSelectorBtn = document.getElementById('ai-selector-btn');
    if (aiSelectorBtn) {
      aiSelectorBtn.addEventListener('click', () => {
        this.hitotone.aiCompanionManager.toggleDropdown();
      });
    }

    const addAiBtn = document.getElementById('add-ai-btn');
    if (addAiBtn) {
      addAiBtn.addEventListener('click', () => {
        this.hitotone.aiCompanionManager.toggleDropdown(false);
        const modal = document.getElementById('add-ai-modal');
        if (modal) this.hitotone.showModal(modal);
      });
    }

    const cancelAddAiBtn = document.getElementById('cancel-add-ai-btn');
    if (cancelAddAiBtn) {
      cancelAddAiBtn.addEventListener('click', () => {
        const modal = document.getElementById('add-ai-modal');
        const form = document.getElementById('add-ai-form');
        if (modal) this.hitotone.hideModal(modal);
        if (form) form.reset();
      });
    }

    const addAiForm = document.getElementById('add-ai-form');
    if (addAiForm) {
      addAiForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        const nameInput = document.getElementById('ai-name');
        const urlInput = document.getElementById('ai-url');
        const name = nameInput ? nameInput.value : '';
        const url = urlInput ? urlInput.value : '';

        this.hitotone.aiServices = await invoke('add_ai_service', {
          service: { id: '', name, url, isDefault: false },
        });
        this.hitotone.aiCompanionManager.renderDropdown();

        const modal = document.getElementById('add-ai-modal');
        if (modal) this.hitotone.hideModal(modal);
        if (addAiForm) addAiForm.reset();
      });
    }

    // ドロップダウン外クリックで閉じる
    document.addEventListener('click', (e) => {
      const selector = document.getElementById('ai-selector');
      const dropdown = document.getElementById('ai-dropdown');
      if (
        selector &&
        dropdown &&
        !selector.contains(e.target) &&
        !dropdown.classList.contains('hidden')
      ) {
        this.hitotone.aiCompanionManager.toggleDropdown(false);
      }
    });
  }

  setupServiceEvents() {
    const addServiceBtn = document.getElementById('add-service-btn');
    if (addServiceBtn) {
      addServiceBtn.addEventListener('click', () => {
        // Close settings first, then open add-service
        this.hitotone.settingsManager.close();
        const modal = document.getElementById('add-service-modal');
        if (modal) this.hitotone.showModal(modal);
      });
    }

    const onboardingAddServiceBtn = document.getElementById('onboarding-add-service-btn');
    if (onboardingAddServiceBtn) {
      onboardingAddServiceBtn.addEventListener('click', () => {
        const modal = document.getElementById('add-service-modal');
        if (modal) this.hitotone.showModal(modal);
      });
    }

    const presetBtns = document.querySelectorAll('.preset-service-btn');
    const submitPresetsBtn = document.getElementById('onboarding-submit-presets-btn');

    presetBtns.forEach((btn) => {
      btn.addEventListener('click', () => {
        btn.classList.toggle('selected');
        if (submitPresetsBtn) {
          const anySelected = Array.from(presetBtns).some((b) => b.classList.contains('selected'));
          submitPresetsBtn.disabled = !anySelected;
        }
      });
    });

    if (submitPresetsBtn) {
      submitPresetsBtn.addEventListener('click', async () => {
        submitPresetsBtn.disabled = true;
        const selectedBtns = Array.from(presetBtns).filter((b) => b.classList.contains('selected'));
        if (selectedBtns.length === 0) return;

        let firstNewServiceId = null;

        for (const btn of selectedBtns) {
          const name = btn.dataset.presetName;
          const url = btn.dataset.presetUrl;
          const icon = btn.dataset.presetIcon;

          this.hitotone.services = await invoke('add_service', {
            service: { id: '', name, url, icon, enabled: true },
          });

          const newService = this.hitotone.services[this.hitotone.services.length - 1];
          if (newService) {
            await invoke('create_service_webview', {
              serviceId: newService.id,
              url: newService.url,
            });
            if (!firstNewServiceId) {
              firstNewServiceId = newService.id;
            }
          }
        }

        const onboarding = document.getElementById('onboarding-screen');
        if (onboarding) this.hitotone.hideModal(onboarding);

        if (firstNewServiceId) {
          await this.hitotone.webViewManager.switchService(firstNewServiceId);
        }

        // Reset selections
        presetBtns.forEach((b) => b.classList.remove('selected'));
        submitPresetsBtn.disabled = true;
      });
    }

    const cancelAddBtn = document.getElementById('cancel-add-btn');
    if (cancelAddBtn) {
      cancelAddBtn.addEventListener('click', () => {
        const modal = document.getElementById('add-service-modal');
        const form = document.getElementById('add-service-form');
        if (modal) this.hitotone.hideModal(modal);
        if (form) form.reset();
      });
    }

    const addServiceForm = document.getElementById('add-service-form');
    if (addServiceForm) {
      addServiceForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        const nameInput = document.getElementById('service-name');
        const urlInput = document.getElementById('service-url');
        const iconInput = document.getElementById('service-icon');
        const name = nameInput ? nameInput.value : '';
        const url = urlInput ? urlInput.value : '';
        const icon = iconInput ? iconInput.value || '🔗' : '🔗';

        this.hitotone.services = await invoke('add_service', {
          service: { id: '', name, url, icon, enabled: true },
        });

        // 新しいサービスのWebViewを作成
        const newService = this.hitotone.services[this.hitotone.services.length - 1];
        if (newService) {
          await invoke('create_service_webview', {
            serviceId: newService.id,
            url: newService.url,
          });

          // 初めてのサービス追加の場合、オンボーディングを隠してそのサービスに切り替え
          if (this.hitotone.services.length === 1) {
            const onboarding = document.getElementById('onboarding-screen');
            if (onboarding) this.hitotone.hideModal(onboarding);
            await this.hitotone.webViewManager.switchService(newService.id);
          }
        }

        const modal = document.getElementById('add-service-modal');
        if (modal) this.hitotone.hideModal(modal);
        if (addServiceForm) addServiceForm.reset();
      });
    }

    const cancelEditBtn = document.getElementById('cancel-edit-btn');
    if (cancelEditBtn) {
      cancelEditBtn.addEventListener('click', () => {
        const modal = document.getElementById('edit-service-modal');
        if (modal) this.hitotone.hideModal(modal);
      });
    }

    const editServiceForm = document.getElementById('edit-service-form');
    if (editServiceForm) {
      editServiceForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        const idInput = document.getElementById('edit-service-id');
        const nameInput = document.getElementById('edit-service-name');
        const urlInput = document.getElementById('edit-service-url');
        const iconInput = document.getElementById('edit-service-icon');

        const id = idInput ? idInput.value : '';
        const name = nameInput ? nameInput.value : '';
        const url = urlInput ? urlInput.value : '';
        const icon = iconInput ? iconInput.value || '🔗' : '🔗';

        const service = this.hitotone.services.find((s) => s.id === id);
        if (service) {
          const updatedService = { ...service, name, url, icon };
          this.hitotone.services = await invoke('update_service', { service: updatedService });

          // WebViewを再作成（URLが変わった可能性があるため）
          await invoke('remove_service_webview', { serviceId: id });
          await invoke('create_service_webview', { serviceId: id, url });

          if (this.hitotone.activeServiceId === id) {
            await this.hitotone.webViewManager.switchService(id);
          }
        }

        const modal = document.getElementById('edit-service-modal');
        if (modal) this.hitotone.hideModal(modal);
        this.hitotone.settingsManager.open();
      });
    }

    const toggleAiBtn = document.getElementById('toggle-ai-btn');
    if (toggleAiBtn) {
      toggleAiBtn.addEventListener('click', () => {
        this.hitotone.aiCompanionManager.toggle();
      });
    }

    const closeAiBtn = document.getElementById('close-ai-btn');
    if (closeAiBtn) {
      closeAiBtn.addEventListener('click', () => {
        this.hitotone.aiCompanionManager.toggle();
      });
    }

    const settingsBtn = document.getElementById('settings-btn');
    if (settingsBtn) {
      settingsBtn.addEventListener('click', () => {
        this.hitotone.settingsManager.open();
      });
    }

    const closeSettingsBtn = document.getElementById('close-settings-btn');
    if (closeSettingsBtn) {
      closeSettingsBtn.addEventListener('click', () => {
        this.hitotone.settingsManager.close();
      });
    }
  }

  setupModalEvents() {
    document.querySelectorAll('.modal').forEach((modal) => {
      modal.addEventListener('click', (e) => {
        if (e.target === modal) {
          this.hitotone.hideModal(modal);
        }
      });
    });
  }

  setupKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
      // Cmd/Ctrl + 数字でサービス切り替え
      if ((e.metaKey || e.ctrlKey) && e.key >= '1' && e.key <= '9') {
        const index = parseInt(e.key) - 1;
        const enabledServices = this.hitotone.services.filter((s) => s.enabled);
        if (index < enabledServices.length) {
          this.hitotone.webViewManager.switchService(enabledServices[index].id);
        }
      }

      // Escape でモーダルを閉じる
      if (e.key === 'Escape') {
        document.querySelectorAll('.modal:not(.hidden)').forEach((modal) => {
          this.hitotone.hideModal(modal);
        });
      }
    });
  }

  setupTauriListeners() {
    // バッジ更新をリッスン
    listen('badge-updated', (event) => {
      const { serviceId, count } = event.payload;
      this.hitotone.paneTreeManager.updateBadge(serviceId, count);
    });

    // AIに送るをリッスン
    listen('send-to-ai', (event) => {
      this.hitotone.aiCompanionManager.sendToAi(event.payload);
    });

    // 認証完了時
    listen('auth-completed', (event) => {
      this.hitotone.webViewManager.navigateActiveWebview(event.payload);
    });
  }
}
