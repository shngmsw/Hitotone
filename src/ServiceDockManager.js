// ServiceDockManager.js - サービスドック管理（バッジ、ドラッグ＆ドロップ） (Tauri v2)

import { invoke } from '@tauri-apps/api/core';

export class ServiceDockManager {
  constructor(hitotone) {
    this.hitotone = hitotone;
    this.badges = new Map();
    this.draggedElement = null;
    this.dropPosition = null;
  }

  render() {
    const serviceList = document.getElementById('service-list');
    if (!serviceList) return;

    serviceList.innerHTML = '';

    this.hitotone.services
      .filter((s) => s.enabled)
      .forEach((service) => {
        const item = document.createElement('div');
        item.className = 'service-item';
        item.dataset.serviceId = service.id;
        item.title = service.name;
        item.draggable = true;

        // Use favicon if available, otherwise use emoji
        if (service.faviconUrl || service.favicon_url) {
          const img = document.createElement('img');
          img.className = 'service-favicon';
          img.src = service.faviconUrl || service.favicon_url;
          img.alt = service.name;
          item.appendChild(img);
        } else {
          item.textContent = service.icon;
        }

        const badge = document.createElement('span');
        badge.className = 'badge hidden';
        badge.textContent = '0';
        item.appendChild(badge);

        item.addEventListener('click', () =>
          this.hitotone.webViewManager.switchService(service.id),
        );

        // Drag and drop event listeners
        item.addEventListener('dragstart', (e) => this.handleDragStart(e));
        item.addEventListener('dragover', (e) => this.handleDragOver(e));
        item.addEventListener('drop', (e) => this.handleDrop(e));
        item.addEventListener('dragend', (e) => this.handleDragEnd(e));
        item.addEventListener('dragenter', (e) => this.handleDragEnter(e));
        item.addEventListener('dragleave', (e) => this.handleDragLeave(e));

        serviceList.appendChild(item);
      });
  }

  updateBadge(serviceId, count) {
    this.badges.set(serviceId, count);
    const item = document.querySelector(`.service-item[data-service-id="${serviceId}"]`);
    if (item) {
      const badge = item.querySelector('.badge');
      if (count > 0) {
        badge.textContent = count > 99 ? '99+' : count;
        badge.classList.remove('hidden');
      } else {
        badge.classList.add('hidden');
      }
    }
  }

  updateServiceIcon(serviceId, faviconUrl) {
    const item = document.querySelector(`.service-item[data-service-id="${serviceId}"]`);
    if (!item) return;

    const service = this.hitotone.services.find((s) => s.id === serviceId);

    // Remove old icon (emoji or old favicon)
    const oldIcon = item.querySelector('.service-favicon') || item.firstChild;
    if (oldIcon && oldIcon.nodeType === Node.TEXT_NODE) {
      oldIcon.remove();
    } else if (oldIcon && oldIcon.classList && oldIcon.classList.contains('service-favicon')) {
      oldIcon.remove();
    }

    // Add new favicon
    const img = document.createElement('img');
    img.className = 'service-favicon';
    img.src = faviconUrl;
    img.alt = service ? service.name : '';

    // 読み込み失敗時はemojiにフォールバック
    img.onerror = () => {
      img.remove();
      if (service) {
        const badge = item.querySelector('.badge');
        const textNode = document.createTextNode(service.icon);
        item.insertBefore(textNode, badge);
        // 保存されたfaviconUrlをクリア
        service.faviconUrl = null;
        service.favicon_url = null;
        invoke('update_service', { service });
      }
    };

    // Insert before badge
    const badge = item.querySelector('.badge');
    item.insertBefore(img, badge);
  }

  restoreBadgesAndActiveState() {
    // バッジを復元
    this.badges.forEach((count, serviceId) => {
      this.updateBadge(serviceId, count);
    });

    // アクティブ状態を復元
    if (this.hitotone.activeServiceId) {
      const activeItem = document.querySelector(
        `.service-item[data-service-id="${this.hitotone.activeServiceId}"]`,
      );
      if (activeItem) {
        activeItem.classList.add('active');
      }
    }
  }

  // Drag and drop handlers
  handleDragStart(e) {
    this.draggedElement = e.currentTarget;
    this.dropPosition = null;

    requestAnimationFrame(() => {
      e.currentTarget.classList.add('dragging');
    });

    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', e.currentTarget.dataset.serviceId);
  }

  handleDragOver(e) {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';

    const target = e.currentTarget;
    if (target === this.draggedElement) {
      return false;
    }

    const rect = target.getBoundingClientRect();
    const midY = rect.top + rect.height / 2;
    const isAbove = e.clientY < midY;

    target.classList.remove('drag-over-above', 'drag-over-below');
    target.classList.add(isAbove ? 'drag-over-above' : 'drag-over-below');
    this.dropPosition = isAbove ? 'above' : 'below';

    return false;
  }

  handleDragEnter(_e) {
    // dragoverで処理するため、ここでは何もしない
  }

  handleDragLeave(e) {
    e.currentTarget.classList.remove('drag-over-above', 'drag-over-below');
  }

  async handleDrop(e) {
    e.stopPropagation();
    e.preventDefault();

    const dropTarget = e.currentTarget;
    dropTarget.classList.remove('drag-over-above', 'drag-over-below');

    if (!this.draggedElement) {
      return false;
    }

    if (this.draggedElement !== dropTarget) {
      const draggedId = this.draggedElement.dataset.serviceId;
      const targetId = dropTarget.dataset.serviceId;

      const draggedIndex = this.hitotone.services.findIndex((s) => s.id === draggedId);
      let targetIndex = this.hitotone.services.findIndex((s) => s.id === targetId);

      if (draggedIndex !== -1 && targetIndex !== -1) {
        if (this.dropPosition === 'below') {
          targetIndex += 1;
        }

        if (draggedIndex < targetIndex) {
          targetIndex -= 1;
        }

        const [draggedService] = this.hitotone.services.splice(draggedIndex, 1);
        this.hitotone.services.splice(targetIndex, 0, draggedService);

        await invoke('reorder_services', { services: this.hitotone.services });

        this.render();
        this.restoreBadgesAndActiveState();
      }
    }

    return false;
  }

  handleDragEnd(e) {
    e.currentTarget.classList.remove('dragging');

    document.querySelectorAll('.service-item').forEach((item) => {
      item.classList.remove('drag-over-above', 'drag-over-below');
    });

    this.draggedElement = null;
    this.dropPosition = null;
  }
}
