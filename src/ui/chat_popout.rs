//! Teams chat popout URL classification.

use reqwest::Url;

fn trusted_teams_url(raw_url: &str) -> Option<Url> {
    let Ok(url) = Url::parse(raw_url) else {
        return None;
    };
    if url.scheme() != "https" {
        return None;
    }

    let host = url.host_str()?;
    if host != "teams.microsoft.com"
        && !host.ends_with(".teams.microsoft.com")
        && host != "teams.live.com"
    {
        return None;
    }

    Some(url)
}

pub fn is_trusted_teams_url(raw_url: &str) -> bool {
    trusted_teams_url(raw_url).is_some()
}

pub fn is_teams_chat_url(raw_url: &str) -> bool {
    let Some(url) = trusted_teams_url(raw_url) else {
        return false;
    };

    let path = url.path().to_ascii_lowercase();
    if path.split('/').any(|segment| {
        matches!(
            segment,
            "channel" | "meet" | "meeting" | "call" | "meetup" | "meetup-join"
        )
    }) {
        return false;
    }

    if path.starts_with("/l/chat/") || path.starts_with("/chat/") {
        return true;
    }
    if !matches!(path.as_str(), "/v2" | "/v2/") {
        return false;
    }

    let mut has_users = false;
    let mut has_chat_context = false;
    let mut has_chat_id = false;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "users" if !value.is_empty() => has_users = true,
            "ctx" if value == "chat" => has_chat_context = true,
            "chatId" if !value.is_empty() => has_chat_id = true,
            _ => {}
        }
    }

    has_users || (has_chat_context && has_chat_id)
}

pub fn get_chat_popout_script() -> String {
    r#"
(function () {
    'use strict';

    const READY_ATTRIBUTE = 'data-rteams-chat-popout-ready';
    const WARNED_ATTRIBUTE = 'data-rteams-chat-popout-warned';
    const ACTION_ATTRIBUTE = 'data-rteams-chat-popout-action';
    const STYLE_ID = 'rteams-chat-popout-style';
    const ROW_CLASS = 'rteams-chat-popout-row';
    const BUTTON_CLASS = 'rteams-chat-popout-button';
    const PORTAL_CLASS = 'rteams-chat-popout-portal';
    const PORTAL_VISIBLE_CLASS = 'rteams-chat-popout-portal-visible';
    const DEBUG = false;
    function dbg() {
        if (DEBUG) console.log.apply(console, ['[RTeams]'].concat(Array.prototype.slice.call(arguments)));
    }
    const FOCUSABLE_SELECTOR = [
        'a[href]',
        'button',
        'input',
        'select',
        'textarea',
        '[tabindex]',
        '[contenteditable]:not([contenteditable="false"])'
    ].join(',');
    const ROW_SELECTOR = [
        '[data-tid="chat-item"]',
        '[data-tid^="chat-item-"]',
        '[role="listitem"][data-tid*="chat-item"]',
        '[role="option"][data-tid*="chat-item"]',
        '[data-automationid^="chat"]',
        '[data-testid^="chat"]',
        '[data-tid*="conversation"]'
    ].join(',');
    const SEMANTIC_ROW_SELECTOR = '[role="listitem"], [role="option"]';
    const MARKED_ROW_SELECTOR = `[${READY_ATTRIBUTE}], [${WARNED_ATTRIBUTE}], [${ACTION_ATTRIBUTE}]`;
    const INVALIDATING_ATTRIBUTES = new Set(['href', 'data-tid', 'role']);
    const OBSERVER_OPTIONS = {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ['href', 'data-tid', 'role', 'class', 'style']
    };
    const pendingRows = new Set();
    const actionByRow = new WeakMap();
    const rowByActionButton = new WeakMap();
    const originalPositionByRow = new WeakMap();
    const positionOptOutRows = new WeakSet();
    let observer = null;
    let flushTimer = null;

    if (window.__rteamsChatPopoutInstalled) {
        return;
    }
    window.__rteamsChatPopoutInstalled = true;

    function hasNonEmptyQueryValue(url, key) {
        return url.searchParams.getAll(key).some(function (value) {
            return value.length > 0;
        });
    }

    function isV2ChatQuery(url, path) {
        if (path !== '/v2' && path !== '/v2/') {
            return false;
        }

        const hasUsers = hasNonEmptyQueryValue(url, 'users');
        const hasChatContext = url.searchParams.getAll('ctx').includes('chat');
        const hasChatId = hasNonEmptyQueryValue(url, 'chatId');
        return hasUsers || (hasChatContext && hasChatId);
    }

    function isChatUrl(url) {
        if (url.protocol !== 'https:') {
            return false;
        }

        const host = url.hostname.toLowerCase();
        if (host !== 'teams.microsoft.com' &&
            !host.endsWith('.teams.microsoft.com') &&
            host !== 'teams.live.com') {
            return false;
        }

        const path = url.pathname.toLowerCase();
        const rejectedPath = path.split('/').some(function (segment) {
            return segment === 'channel' ||
                segment === 'meet' ||
                segment === 'meeting' ||
                segment === 'call' ||
                segment === 'meetup' ||
                segment === 'meetup-join';
        });
        if (rejectedPath) {
            return false;
        }

        if (path.startsWith('/l/chat/') || path.startsWith('/chat/')) {
            return true;
        }
        return isV2ChatQuery(url, path);
    }

    function canonicalLogicalRow(candidate) {
        if (!(candidate instanceof Element)) {
            return null;
        }

        if (candidate.matches('a') && candidate.matches(ROW_SELECTOR)) {
            return candidate;
        }
        const containingAnchor = candidate.closest('a');
        if (containingAnchor && containingAnchor.matches(ROW_SELECTOR)) {
            return containingAnchor;
        }
        if (candidate.matches(ROW_SELECTOR) &&
            Array.from(candidate.querySelectorAll('a')).some(function (anchor) {
                return anchor.matches(ROW_SELECTOR);
            })) {
            return null;
        }

        const semanticRow = candidate.closest('[role="listitem"], [role="option"]');
        if (semanticRow &&
            (semanticRow.matches(ROW_SELECTOR) || semanticRow.querySelector(ROW_SELECTOR))) {
            return semanticRow;
        }

        let outermostMatch = null;
        let current = candidate;
        while (current) {
            if (current.matches(ROW_SELECTOR)) {
                outermostMatch = current;
            }
            current = current.parentElement;
        }
        return outermostMatch;
    }

    function interactionTrigger(row) {
        return row.closest('a[href], button, [role="button"]') || row;
    }

    function resolveRowUrl(row) {
        const anchors = new Set();
        const trigger = interactionTrigger(row);
        if (trigger.matches('a[href]')) {
            anchors.add(trigger);
        }
        if (row.matches('a[href]')) {
            anchors.add(row);
        }
        row.querySelectorAll('a[href]').forEach(function (anchor) {
            anchors.add(anchor);
        });

        for (const anchor of anchors) {
            try {
                const url = new URL(anchor.getAttribute('href'), location.origin);
                if (isChatUrl(url)) {
                    return url.href;
                }
            } catch (_error) {
                continue;
            }
        }
        return null;
    }

    function findDirectButton(row) {
        return Array.from(row.children).find(function (child) {
            return child.classList.contains(BUTTON_CLASS);
        });
    }

    function actionButton(row) {
        const action = actionByRow.get(row);
        if (action && action.button.isConnected) {
            return action.button;
        }
        if (action) {
            removeRowAction(row);
        }
        return findDirectButton(row);
    }

    function warnInvalidRow(row) {
        if (!row.hasAttribute(WARNED_ATTRIBUTE)) {
            row.setAttribute(WARNED_ATTRIBUTE, 'true');
            console.warn('[RTeams] Chat row has no valid Teams chat URL.', row);
        }
    }

    function stillOwnsRelativePosition(row) {
        return row.style.getPropertyValue('position') === 'relative' &&
            row.style.getPropertyPriority('position') === '';
    }

    function restoreOriginalPosition(row) {
        const original = originalPositionByRow.get(row);
        if (!original) {
            return;
        }

        if (stillOwnsRelativePosition(row)) {
            if (original.value) {
                row.style.setProperty('position', original.value, original.priority);
            } else {
                row.style.removeProperty('position');
            }
        } else {
            positionOptOutRows.add(row);
        }
        originalPositionByRow.delete(row);
    }

    function clearDecoration(row) {
        removeRowAction(row);
        row.removeAttribute(READY_ATTRIBUTE);
        row.classList.remove(ROW_CLASS);
        restoreOriginalPosition(row);
    }

    function ensurePositioningContainer(row) {
        const original = originalPositionByRow.get(row);
        if (original) {
            if (!stillOwnsRelativePosition(row)) {
                originalPositionByRow.delete(row);
                positionOptOutRows.add(row);
                return getComputedStyle(row).position !== 'static';
            }
            return true;
        }

        const computedPosition = getComputedStyle(row).position;
        if (positionOptOutRows.has(row)) {
            return computedPosition !== 'static';
        }
        if (computedPosition === 'static') {
            originalPositionByRow.set(row, {
                value: row.style.getPropertyValue('position'),
                priority: row.style.getPropertyPriority('position')
            });
            row.style.setProperty('position', 'relative');
        }
        return true;
    }

    function shouldUsePortalAction(row) {
        if (hasInteractivePortalConstraint(row)) {
            restoreOriginalPosition(row);
            return true;
        }
        return !ensurePositioningContainer(row);
    }

    function hasInteractivePortalConstraint(row) {
        return Boolean(row.closest('a[href], button, [role="button"]')) ||
            row.matches('[role="option"], [role="button"]');
    }

    function restoreRowClass(row) {
        if (!row.classList.contains(ROW_CLASS)) {
            row.classList.add(ROW_CLASS);
        }
    }

    function createButton(row) {
        const button = document.createElement('button');
        button.type = 'button';
        button.className = BUTTON_CLASS;
        button.title = 'Open chat in a new window';
        button.setAttribute('aria-label', 'Open chat in a new window');
        button.innerHTML = '<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M9 2h5v5h-1.5V4.56L7.53 9.53 6.47 8.47 11.44 3.5H9V2Z"/><path d="M3.5 4.5H7V6H3.5v6.5H10V9h1.5v3.5A1.5 1.5 0 0 1 10 14H3.5A1.5 1.5 0 0 1 2 12.5V6a1.5 1.5 0 0 1 1.5-1.5Z"/></svg>';

        ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'keydown', 'keyup'].forEach(function (eventName) {
            button.addEventListener(eventName, function (event) {
                event.stopPropagation();
            });
        });
        button.addEventListener('click', function (event) {
            event.preventDefault();
            event.stopPropagation();

            const chatUrl = resolveRowUrl(row);
            if (!chatUrl) {
                console.warn('[RTeams] Chat URL is no longer available.', row);
                return;
            }
            window.open(chatUrl, '_blank', 'popup=yes');
        });

        return button;
    }

    function addActionListener(action, target, eventName, listener, options) {
        target.addEventListener(eventName, listener, options);
        action.listeners.push({ target, eventName, listener, options });
    }

    function updatePortalPosition(action) {
        if (!action.row.isConnected) {
            removeRowAction(action.row);
            return;
        }

        const rect = action.trigger.getBoundingClientRect();
        const left = Math.max(4, Math.min(window.innerWidth - 28, rect.right - 32));
        const top = Math.max(4, Math.min(window.innerHeight - 28, rect.top + (rect.height - 24) / 2));
        action.button.style.left = `${left}px`;
        action.button.style.top = `${top}px`;
    }

    function showPortalAction(action) {
        if (action.hideTimer !== null) {
            clearTimeout(action.hideTimer);
            action.hideTimer = null;
        }
        updatePortalPosition(action);
        action.button.classList.add(PORTAL_VISIBLE_CLASS);
    }

    function portalActionIsActive(action) {
        return action.trigger.matches(':hover') ||
            action.trigger.matches(':focus-within') ||
            action.button.matches(':hover') ||
            document.activeElement === action.button;
    }

    function forwardTabToPortal(action, event) {
        if (event.key !== 'Tab' || event.shiftKey || event.altKey ||
            event.ctrlKey || event.metaKey) {
            return;
        }

        event.preventDefault();
        event.stopPropagation();
        if (document.activeElement instanceof HTMLElement &&
            (document.activeElement === action.trigger ||
                action.trigger.contains(document.activeElement))) {
            action.returnFocusElement = document.activeElement;
        }
        showPortalAction(action);
        action.button.focus();
    }

    function returnFocusToRow(action, event) {
        if (event.key !== 'Tab' || !event.shiftKey || event.altKey ||
            event.ctrlKey || event.metaKey) {
            return;
        }

        event.preventDefault();
        event.stopPropagation();
        const remembered = action.returnFocusElement;
        const focusTarget = remembered && remembered.isConnected &&
            (remembered === action.trigger || action.trigger.contains(remembered))
            ? remembered
            : action.trigger;
        focusTarget.focus();
    }

    function isEligibleFocusable(element) {
        if (!(element instanceof HTMLElement) ||
            element.classList.contains(PORTAL_CLASS) ||
            element.tabIndex < 0 ||
            element.matches(':disabled') ||
            element.getAttribute('aria-disabled') === 'true' ||
            element.closest('[inert], [aria-hidden="true"]')) {
            return false;
        }

        const style = getComputedStyle(element);
        return !element.hidden &&
            style.display !== 'none' &&
            style.visibility !== 'hidden' &&
            style.visibility !== 'collapse' &&
            element.getClientRects().length > 0;
    }

    function findNextFocusableAfter(origin) {
        if (!(origin instanceof Element) || !origin.isConnected) {
            return null;
        }

        const candidates = document.querySelectorAll(FOCUSABLE_SELECTOR);
        for (const candidate of candidates) {
            if (origin.contains(candidate) ||
                !(origin.compareDocumentPosition(candidate) & Node.DOCUMENT_POSITION_FOLLOWING)) {
                continue;
            }
            if (isEligibleFocusable(candidate)) {
                return candidate;
            }
        }
        return null;
    }

    function focusNextAfterOrigin(action, event) {
        if (event.key !== 'Tab' || event.shiftKey || event.altKey ||
            event.ctrlKey || event.metaKey) {
            return;
        }

        const nextFocusable = findNextFocusableAfter(action.trigger);
        if (!nextFocusable) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        nextFocusable.focus();
    }

    function schedulePortalHide(action) {
        if (action.hideTimer !== null) {
            clearTimeout(action.hideTimer);
        }
        action.hideTimer = setTimeout(function () {
            action.hideTimer = null;
            if (portalActionIsActive(action)) {
                showPortalAction(action);
            } else {
                action.button.classList.remove(PORTAL_VISIBLE_CLASS);
            }
        }, 0);
    }

    function removeRowAction(row) {
        const action = actionByRow.get(row);
        if (action) {
            if (action.hideTimer !== null) {
                clearTimeout(action.hideTimer);
            }
            action.listeners.forEach(function (entry) {
                entry.target.removeEventListener(
                    entry.eventName,
                    entry.listener,
                    entry.options
                );
            });
            rowByActionButton.delete(action.button);
            action.button.remove();
            actionByRow.delete(row);
        } else {
            const button = findDirectButton(row);
            if (button) {
                button.remove();
            }
        }
        row.removeAttribute(ACTION_ATTRIBUTE);
    }

    function registerRowAction(row, button, portal, trigger) {
        const action = {
            row,
            trigger,
            button,
            portal,
            listeners: [],
            hideTimer: null,
            returnFocusElement: null
        };
        actionByRow.set(row, action);
        rowByActionButton.set(button, row);
        row.setAttribute(ACTION_ATTRIBUTE, 'true');
        return action;
    }

    function createInlineAction(row) {
        const button = createButton(row);
        row.appendChild(button);
        registerRowAction(row, button, false, row);
        return button;
    }

    function createPortalAction(row) {
        const trigger = interactionTrigger(row);
        const button = createButton(row);
        button.classList.add(PORTAL_CLASS);
        document.body.appendChild(button);

        const action = registerRowAction(row, button, true, trigger);
        const show = function () {
            showPortalAction(action);
        };
        const hide = function () {
            schedulePortalHide(action);
        };
        const rememberFocus = function (event) {
            action.returnFocusElement = event.target;
            showPortalAction(action);
        };
        const reposition = function () {
            if (button.classList.contains(PORTAL_VISIBLE_CLASS)) {
                updatePortalPosition(action);
            }
        };

        addActionListener(action, trigger, 'pointerenter', show);
        addActionListener(action, trigger, 'pointerleave', hide);
        addActionListener(action, trigger, 'focusin', rememberFocus);
        addActionListener(action, trigger, 'focusout', hide);
        addActionListener(action, trigger, 'keydown', function (event) {
            forwardTabToPortal(action, event);
        });
        addActionListener(action, button, 'pointerenter', show);
        addActionListener(action, button, 'pointerleave', hide);
        addActionListener(action, button, 'focusin', show);
        addActionListener(action, button, 'focusout', hide);
        addActionListener(action, button, 'keydown', function (event) {
            returnFocusToRow(action, event);
            focusNextAfterOrigin(action, event);
        });
        addActionListener(action, window, 'scroll', reposition, true);
        addActionListener(action, window, 'resize', reposition);

        if (trigger.matches(':hover') || trigger.matches(':focus-within')) {
            showPortalAction(action);
        }
        return button;
    }

    function ensureRowAction(row, shouldUsePortal) {
        const action = actionByRow.get(row);
        const expectedTrigger = shouldUsePortal ? interactionTrigger(row) : row;
        if (action && action.button.isConnected && action.portal === shouldUsePortal &&
            action.trigger === expectedTrigger) {
            return action.button;
        }

        removeRowAction(row);
        return shouldUsePortal ? createPortalAction(row) : createInlineAction(row);
    }

    function decorateRow(row) {
        const existingButton = actionButton(row);
        if (row.hasAttribute(READY_ATTRIBUTE) && existingButton) {
            restoreRowClass(row);
            ensureRowAction(row, shouldUsePortalAction(row));
            row.removeAttribute(WARNED_ATTRIBUTE);
            return;
        }

        if (!resolveRowUrl(row)) {
            clearDecoration(row);
            warnInvalidRow(row);
            return;
        }

        row.removeAttribute(WARNED_ATTRIBUTE);
        restoreRowClass(row);
        ensureRowAction(row, shouldUsePortalAction(row));
        row.setAttribute(READY_ATTRIBUTE, 'true');
    }

    function ensureStyle() {
        if (document.getElementById(STYLE_ID)) {
            return;
        }

        const style = document.createElement('style');
        style.id = STYLE_ID;
        style.textContent = `
            .${BUTTON_CLASS} {
                width: 24px;
                height: 24px;
                display: inline-flex;
                align-items: center;
                justify-content: center;
                padding: 4px;
                border: 0;
                border-radius: 4px;
                color: currentColor;
                background: rgba(255, 255, 255, 0.9);
                cursor: pointer;
            }
            .${ROW_CLASS} > .${BUTTON_CLASS} {
                position: absolute;
                top: 50%;
                right: 8px;
                z-index: 2;
                opacity: 0;
                pointer-events: none;
                transform: translateY(-50%);
            }
            .${ROW_CLASS}:hover > .${BUTTON_CLASS},
            .${ROW_CLASS}:focus-within > .${BUTTON_CLASS} {
                opacity: 1;
                pointer-events: auto;
            }
            .${BUTTON_CLASS}:focus-visible {
                outline: 2px solid currentColor;
                outline-offset: 2px;
            }
            .${BUTTON_CLASS}.${PORTAL_CLASS} {
                position: fixed;
                z-index: 2147483647;
                visibility: hidden;
                opacity: 0;
                pointer-events: none;
            }
            .${BUTTON_CLASS}.${PORTAL_CLASS}.${PORTAL_VISIBLE_CLASS} {
                visibility: visible;
                opacity: 1;
                pointer-events: auto;
            }
            .${BUTTON_CLASS} svg {
                display: block;
                width: 16px;
                height: 16px;
                fill: currentColor;
            }
        `;
        (document.head || document.documentElement).appendChild(style);
    }

    function decorateInitialRows() {
        const rows = new Set();
        var candidates = document.querySelectorAll(ROW_SELECTOR);
        dbg('Found', candidates.length, 'candidates with ROW_SELECTOR');
        candidates.forEach(function (candidate) {
            const row = canonicalLogicalRow(candidate);
            if (row) {
                rows.add(row);
            }
        });
        dbg('Decorating', rows.size, 'initial rows');
        rows.forEach(decorateRow);
    }

    function invalidateAndEnqueueRow(row) {
        if (!row) {
            return;
        }
        row.removeAttribute(READY_ATTRIBUTE);
        pendingRows.add(row);
    }

    function enqueueMutationTarget(target) {
        if (!(target instanceof Element)) {
            return;
        }

        const rows = new Set();
        const markedRow = target.closest(MARKED_ROW_SELECTOR);
        const canonicalRow = canonicalLogicalRow(target);
        if (markedRow) {
            rows.add(markedRow);
        }
        if (canonicalRow) {
            rows.add(canonicalRow);
        }
        rows.forEach(invalidateAndEnqueueRow);
    }

    function enqueueRowsInAddedNode(node) {
        if (!(node instanceof Element)) {
            return;
        }

        const rows = new Set();
        if (node.matches(MARKED_ROW_SELECTOR)) {
            rows.add(node);
        }
        node.querySelectorAll(MARKED_ROW_SELECTOR).forEach(function (row) {
            rows.add(row);
        });

        const candidates = [];
        if (node.matches(ROW_SELECTOR)) {
            candidates.push(node);
        }
        candidates.push(...node.querySelectorAll(ROW_SELECTOR));
        candidates.forEach(function (candidate) {
            const row = canonicalLogicalRow(candidate);
            if (row) {
                rows.add(row);
            }
        });
        rows.forEach(invalidateAndEnqueueRow);
    }

    function invalidateDescendantRowsForAttributeMutation(target) {
        const rows = new Set();
        if (target.matches(MARKED_ROW_SELECTOR)) {
            rows.add(target);
        }
        target.querySelectorAll(MARKED_ROW_SELECTOR).forEach(function (row) {
            rows.add(row);
        });

        rows.forEach(function (row) {
            clearDecoration(row);
            row.removeAttribute(WARNED_ATTRIBUTE);
            pendingRows.add(row);
        });
        return rows;
    }

    function enqueueRelevantAttributeMutation(target) {
        if (!(target instanceof Element)) {
            return;
        }

        const containingMarkedRow = target.closest(MARKED_ROW_SELECTOR);
        const rows = invalidateDescendantRowsForAttributeMutation(target);
        if (containingMarkedRow) {
            rows.add(containingMarkedRow);
        }

        const candidates = [];
        if (target.matches(ROW_SELECTOR)) {
            candidates.push(target);
        }
        candidates.push(...target.querySelectorAll(ROW_SELECTOR));
        candidates.forEach(function (candidate) {
            const row = canonicalLogicalRow(candidate);
            if (row) {
                rows.add(row);
            }
        });

        const canonicalTarget = canonicalLogicalRow(target);
        if (canonicalTarget) {
            rows.add(canonicalTarget);
        }
        rows.forEach(invalidateAndEnqueueRow);
    }

    function isDecoratedOrOwnedRow(row) {
        return row instanceof Element &&
            (row.hasAttribute(READY_ATTRIBUTE) ||
                row.hasAttribute(ACTION_ATTRIBUTE) ||
                actionByRow.has(row) ||
                originalPositionByRow.has(row));
    }

    function cleanupRemovedNode(node) {
        if (!(node instanceof Element)) {
            return;
        }

        const rows = new Set();
        if (node.matches(MARKED_ROW_SELECTOR)) {
            rows.add(node);
        }
        node.querySelectorAll(MARKED_ROW_SELECTOR).forEach(function (row) {
            rows.add(row);
        });

        const removedActionRow = rowByActionButton.get(node);
        if (removedActionRow) {
            rows.add(removedActionRow);
        }
        node.querySelectorAll(`.${BUTTON_CLASS}`).forEach(function (button) {
            const row = rowByActionButton.get(button);
            if (row) {
                rows.add(row);
            }
        });

        rows.forEach(function (row) {
            if (row.isConnected) {
                invalidateAndEnqueueRow(row);
            } else {
                pendingRows.delete(row);
                clearDecoration(row);
                row.removeAttribute(WARNED_ATTRIBUTE);
            }
        });
    }

    function isCurrentLogicalRow(row) {
        const candidates = [];
        if (row.matches(ROW_SELECTOR)) {
            candidates.push(row);
        }
        candidates.push(...row.querySelectorAll(ROW_SELECTOR));
        return candidates.some(function (candidate) {
            return canonicalLogicalRow(candidate) === row;
        });
    }

    function observeMutations() {
        if (observer && document.body) {
            observer.observe(document.body, OBSERVER_OPTIONS);
        }
    }

    function flushPendingRows() {
        flushTimer = null;
        if (pendingRows.size === 0) {
            return;
        }

        const rows = Array.from(pendingRows);
        pendingRows.clear();
        if (observer) {
            observer.disconnect();
        }
        try {
            rows.forEach(function (row) {
                if (row.isConnected && isCurrentLogicalRow(row)) {
                    decorateRow(row);
                } else {
                    clearDecoration(row);
                    row.removeAttribute(WARNED_ATTRIBUTE);
                }
            });
        } finally {
            observeMutations();
        }
    }

    function schedulePendingFlush() {
        if (pendingRows.size === 0 || flushTimer !== null) {
            return;
        }
        flushTimer = setTimeout(flushPendingRows, 50);
    }

    function handleMutations(mutations) {
        mutations.forEach(function (mutation) {
            if (mutation.type === 'childList') {
                enqueueMutationTarget(mutation.target);
                mutation.addedNodes.forEach(enqueueRowsInAddedNode);
                mutation.removedNodes.forEach(cleanupRemovedNode);
            } else if (mutation.type === 'attributes' &&
                INVALIDATING_ATTRIBUTES.has(mutation.attributeName)) {
                enqueueRelevantAttributeMutation(mutation.target);
            } else if (mutation.type === 'attributes' &&
                (mutation.attributeName === 'class' || mutation.attributeName === 'style') &&
                isDecoratedOrOwnedRow(mutation.target)) {
                enqueueMutationTarget(mutation.target);
            }
        });
        schedulePendingFlush();
    }

    function initialize() {
        console.log('[RTeams] Chat popout script initialized');
        dbg('debug mode enabled');
        ensureStyle();
        decorateInitialRows();

        observer = new MutationObserver(handleMutations);
        observeMutations();
        dbg('MutationObserver started');
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initialize, { once: true });
    } else {
        initialize();
    }
}());
"#
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{get_chat_popout_script, is_teams_chat_url, is_trusted_teams_url};

    #[test]
    fn injection_script_contains_popup_and_deduplication_contracts() {
        let script = get_chat_popout_script();

        assert!(script.contains("data-rteams-chat-popout-ready"));
        assert!(script.contains("MutationObserver"));
        assert!(script.contains("window.open(chatUrl"));
        assert!(script.contains("stopPropagation"));
        assert!(script.contains("aria-label"));
        assert!(script.contains("getAll('ctx').includes('chat')"));
        assert!(script.contains("function hasNonEmptyQueryValue(url, key)"));
        assert!(script.contains("function isV2ChatQuery(url, path)"));
        assert!(script.contains("hasNonEmptyQueryValue(url, 'chatId')"));
        assert!(script.contains("attributes: true"));
        assert!(script.contains("attributeFilter: ['href', 'data-tid', 'role', 'class', 'style']"));
        assert!(script.contains("function canonicalLogicalRow(candidate)"));
        assert!(script.contains("candidate.matches('a') && candidate.matches(ROW_SELECTOR)"));
        assert!(script.contains("let outermostMatch = null"));
        assert!(!script.contains("function safeNonAnchorHost(row)"));
        assert!(script.contains("const actionByRow = new WeakMap()"));
        assert!(script.contains("function createPortalAction(row)"));
        assert!(script.contains("function interactionTrigger(row)"));
        assert!(script.contains("closest('a[href], button, [role=\"button\"]')"));
        assert!(script.contains("const trigger = interactionTrigger(row)"));
        assert!(script.contains("function hasInteractivePortalConstraint(row)"));
        assert!(script.contains("row.matches('[role=\"option\"], [role=\"button\"]')"));
        assert!(script.contains("function forwardTabToPortal(action, event)"));
        assert!(script.contains("function returnFocusToRow(action, event)"));
        assert!(script.contains("function findNextFocusableAfter(origin)"));
        assert!(script.contains("function focusNextAfterOrigin(action, event)"));
        assert!(script.contains("if (!nextFocusable) {\n            return;"));
        assert!(!script.contains("else {\n            action.button.focus();"));
        assert!(script.contains("action.button.focus()"));
        assert!(script.contains("focusTarget.focus()"));
        assert!(script.contains("document.body.appendChild(button)"));
        assert!(script.contains("function removeRowAction(row)"));
        assert!(script.contains("closest('[role=\"listitem\"], [role=\"option\"]')"));
        assert!(script.contains("new Set"));
        assert!(script.contains("'pointerup', 'mouseup'"));
        assert!(script.contains("'keydown', 'keyup'"));
        assert!(script.contains("const pendingRows = new Set()"));
        assert!(script.contains("function invalidateAndEnqueueRow(row)"));
        assert!(script.contains("function enqueueRowsInAddedNode(node)"));
        assert!(script.contains("function invalidateDescendantRowsForAttributeMutation(target)"));
        assert!(script.contains("function enqueueRelevantAttributeMutation(target)"));
        assert!(script.contains("function isCurrentLogicalRow(row)"));
        assert!(script.contains("function flushPendingRows()"));
        assert!(script.contains("function schedulePendingFlush()"));
        assert!(script.contains("mutation.type === 'childList'"));
        assert!(script.contains("mutation.addedNodes.forEach(enqueueRowsInAddedNode)"));
        assert!(script.contains("mutation.removedNodes.forEach(cleanupRemovedNode)"));
        assert!(script.contains("function isDecoratedOrOwnedRow(row)"));
        assert!(!script.contains("scanRows();"));
        assert!(!script.contains("clearTimeout(flushTimer)"));
        assert!(script.contains("const originalPositionByRow = new WeakMap()"));
        assert!(script.contains("const positionOptOutRows = new WeakSet()"));
        assert!(script.contains("computedPosition === 'static'"));
        assert!(script.contains("function stillOwnsRelativePosition(row)"));
        assert!(script.contains("function ensurePositioningContainer(row)"));
        assert!(script.contains("function shouldUsePortalAction(row)"));
        assert!(script.contains("return !ensurePositioningContainer(row)"));
        assert!(script.contains("function restoreOriginalPosition(row)"));
        assert!(script.contains("row.removeAttribute(WARNED_ATTRIBUTE)"));
    }

    #[test]
    fn accepts_teams_chat_urls() {
        assert!(is_teams_chat_url(
            "https://teams.microsoft.com/l/chat/0/0?users=alice@example.com"
        ));
        assert!(is_teams_chat_url(
            "https://teams.microsoft.com/v2/?ctx=chat&chatId=19%3Aabc"
        ));
        assert!(is_teams_chat_url(
            "https://teams.live.com/v2/?users=alice%40example.com"
        ));
        assert!(is_teams_chat_url(
            "https://teams.microsoft.com/v2?users=alice@example.com"
        ));
    }

    #[test]
    fn rejects_non_chat_urls() {
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com/l/channel/19%3Aabc/general"
        ));
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com/meet/123456"
        ));
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com.evil.example/l/chat/0/0?users=a"
        ));
        assert!(!is_teams_chat_url("not a url"));
    }

    #[test]
    fn rejects_non_https_chat_urls() {
        assert!(!is_teams_chat_url(
            "http://teams.microsoft.com/l/chat/0/0?users=a"
        ));
    }

    #[test]
    fn rejects_meetup_join_routes_with_chat_query() {
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com/l/meetup-join/abc?users=a"
        ));
    }

    #[test]
    fn rejects_query_markers_on_non_v2_paths() {
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com/l/person/123?users=alice"
        ));
    }

    #[test]
    fn rejects_chat_context_without_identifier() {
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com/v2/?ctx=chat"
        ));
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com/v2/?ctx=chat&chatId="
        ));
    }

    #[test]
    fn rejects_empty_users_query() {
        assert!(!is_teams_chat_url("https://teams.microsoft.com/v2/?users="));
    }

    #[test]
    fn trusts_only_https_teams_hosts() {
        assert!(is_trusted_teams_url("https://teams.microsoft.com/"));
        assert!(is_trusted_teams_url("https://chat.teams.microsoft.com/"));
        assert!(is_trusted_teams_url("https://teams.live.com/"));
        assert!(!is_trusted_teams_url(
            "https://teams.microsoft.com.evil.example/"
        ));
        assert!(!is_trusted_teams_url("https://chat.teams.live.com/"));
        assert!(!is_trusted_teams_url("http://teams.microsoft.com/"));
    }
}
