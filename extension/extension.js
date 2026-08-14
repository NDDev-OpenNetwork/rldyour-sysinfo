/* rldyour-sysinfo — GNOME Shell panel indicator
 * Copyright (C) 2026 NDDev OpenNetwork
 * SPDX-License-Identifier: AGPL-3.0-or-later
 */

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {Indicator} from './lib/indicator.js';

/** First slot of the centre box, which is the position left of the clock. */
const PANEL_BOX = 'center';
const PANEL_POSITION = 0;

export default class SysinfoExtension extends Extension {
    enable() {
        this._indicator = new Indicator(this.getSettings());
        Main.panel.addToStatusArea(this.uuid, this._indicator, PANEL_POSITION, PANEL_BOX);
    }

    disable() {
        this._indicator.destroy();
        this._indicator = null;
    }
}
