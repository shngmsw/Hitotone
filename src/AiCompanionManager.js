// AiCompanionManager.js - AIコンパニオン管理 (Tauri v2)

import { invoke } from '@tauri-apps/api/core';

export class AiCompanionManager {
  constructor(hitotone) {
    this.hitotone = hitotone;
  }

  // XSS対策: HTMLエスケープ
  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  async setup(show, width) {
    const aiCompanion = document.getElementById('ai-companion');
    if (!aiCompanion) return;

    // 幅を設定（300-800pxの範囲でバリデーション）
    if (width) {
      const minWidth = 300;
      const maxWidth = 800;
      const validWidth = Math.max(minWidth, Math.min(maxWidth, width));
      aiCompanion.style.width = `${validWidth}px`;
    }

    // AIセレクターを初期化
    this.renderDropdown();
    this.updateSelectorDisplay();

    if (show) {
      aiCompanion.classList.remove('hidden');
      // Rust側でAI WebViewを作成
      if (this.hitotone.activeAiService) {
        await invoke('setup_ai_webview', {
          url: this.hitotone.activeAiService.url,
          width: width || 400,
        });
      }
    } else {
      aiCompanion.classList.add('hidden');
    }
  }

  async toggle() {
    const aiCompanion = document.getElementById('ai-companion');
    if (!aiCompanion) return;

    const isHidden = aiCompanion.classList.toggle('hidden');

    // Rust側でトグル＆レイアウト更新
    await invoke('toggle_ai_webview');

    if (!isHidden && this.hitotone.activeAiService) {
      // AI WebViewがまだ作成されていない場合は作成
      try {
        await invoke('create_ai_webview', {
          url: this.hitotone.activeAiService.url,
        });
      } catch (_) {
        // Already created
      }
    }

    // レイアウト更新
    await invoke('update_layout');
  }

  updateSelectorDisplay() {
    const nameElement = document.getElementById('ai-current-name');
    if (nameElement && this.hitotone.activeAiService) {
      nameElement.textContent = this.hitotone.activeAiService.name;
    }
  }

  renderDropdown() {
    const list = document.getElementById('ai-dropdown-list');
    if (!list) return;

    list.innerHTML = this.hitotone.aiServices
      .map((service) => {
        const isActive =
          this.hitotone.activeAiService && service.id === this.hitotone.activeAiService.id;
        const escapedId = this.escapeHtml(service.id);
        const escapedName = this.escapeHtml(service.name);
        const deleteBtn = service.isDefault
          ? ''
          : `<button class="delete-ai-btn" data-id="${escapedId}" title="削除">×</button>`;
        return `
        <div class="ai-dropdown-item ${isActive ? 'active' : ''}" data-id="${escapedId}">
          <span>${escapedName}</span>
          ${deleteBtn}
        </div>
      `;
      })
      .join('');

    // クリックイベントを設定
    list.querySelectorAll('.ai-dropdown-item').forEach((item) => {
      item.addEventListener('click', (e) => {
        if (e.target.classList.contains('delete-ai-btn')) return;
        this.switchService(item.dataset.id);
      });
    });

    // 削除ボタンのイベント
    list.querySelectorAll('.delete-ai-btn').forEach((btn) => {
      btn.addEventListener('click', async (e) => {
        e.stopPropagation();
        await this.removeService(btn.dataset.id);
      });
    });
  }

  async switchService(serviceId) {
    const service = this.hitotone.aiServices.find((s) => s.id === serviceId);
    if (!service) return;

    // Rust側でAIサービスを切り替え（WebViewのナビゲーションも含む）
    this.hitotone.activeAiService = await invoke('switch_ai_service', { serviceId });
    this.updateSelectorDisplay();
    this.renderDropdown();
    this.toggleDropdown(false);
  }

  async removeService(serviceId) {
    this.hitotone.aiServices = await invoke('remove_ai_service', { serviceId });
    this.hitotone.activeAiService = await invoke('get_active_ai_service');
    this.renderDropdown();
    this.updateSelectorDisplay();
  }

  toggleDropdown(show = null) {
    const dropdown = document.getElementById('ai-dropdown');
    if (!dropdown) return;

    if (show === null) {
      dropdown.classList.toggle('hidden');
    } else {
      dropdown.classList.toggle('hidden', !show);
    }
  }

  async sendToAi(text) {
    const aiCompanion = document.getElementById('ai-companion');
    if (aiCompanion && aiCompanion.classList.contains('hidden')) {
      await this.toggle();
    }

    // Rust側でAI WebViewにテキストを送信
    await invoke('send_to_ai_webview', { text });
  }

  setupResizeHandle() {
    const resizeHandle = document.getElementById('resize-handle');
    const aiCompanion = document.getElementById('ai-companion');

    if (!resizeHandle || !aiCompanion) {
      return;
    }

    let isResizing = false;
    let startX = 0;
    let startWidth = 0;
    let animationFrameId = null;

    const computedStyle = getComputedStyle(document.documentElement);
    const minWidth = parseInt(computedStyle.getPropertyValue('--ai-min-width')) || 300;
    const maxWidth = parseInt(computedStyle.getPropertyValue('--ai-max-width')) || 800;

    const onMouseMove = (e) => {
      if (!isResizing || aiCompanion.classList.contains('hidden')) return;

      if (animationFrameId) {
        cancelAnimationFrame(animationFrameId);
      }

      animationFrameId = requestAnimationFrame(() => {
        const deltaX = startX - e.clientX;
        const newWidth = Math.max(minWidth, Math.min(maxWidth, startWidth + deltaX));
        aiCompanion.style.width = `${newWidth}px`;
      });
    };

    const onMouseUp = () => {
      if (isResizing) {
        isResizing = false;
        aiCompanion.classList.remove('resizing');
        document.body.classList.remove('resizing-active');
        document.body.style.cursor = '';
        document.body.style.userSelect = '';

        const currentWidth = aiCompanion.offsetWidth;
        invoke('resize_ai_webview', { width: currentWidth }).catch((err) => {
          console.warn('AI幅の保存に失敗しました:', err);
        });
      }
    };

    resizeHandle.addEventListener('mousedown', (e) => {
      if (aiCompanion.classList.contains('hidden')) {
        return;
      }

      isResizing = true;
      startX = e.clientX;
      startWidth = aiCompanion.offsetWidth;
      aiCompanion.classList.add('resizing');
      document.body.classList.add('resizing-active');
      document.body.style.cursor = 'ew-resize';
      document.body.style.userSelect = 'none';
      e.preventDefault();
    });

    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);

    // キーボードアクセシビリティ
    resizeHandle.addEventListener('keydown', (e) => {
      if (aiCompanion.classList.contains('hidden')) return;

      const step = 10;
      const currentWidth = aiCompanion.offsetWidth;
      let newWidth = currentWidth;

      if (e.key === 'ArrowLeft') {
        newWidth = Math.min(maxWidth, currentWidth + step);
        e.preventDefault();
      } else if (e.key === 'ArrowRight') {
        newWidth = Math.max(minWidth, currentWidth - step);
        e.preventDefault();
      }

      if (newWidth !== currentWidth) {
        aiCompanion.style.width = `${newWidth}px`;
        invoke('resize_ai_webview', { width: newWidth }).catch((err) => {
          console.warn('AI幅の保存に失敗しました:', err);
        });
      }
    });
  }
}
