/* rldyour-sysinfo — panel indicator
 * Copyright (C) 2026 NDDev OpenNetwork
 * SPDX-License-Identifier: AGPL-3.0-or-later
 */

import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';

import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

import {Client} from './client.js';
import {ABSENT, celsius, percent, rate} from './format.js';

/** Panel cells, in display order: caption, then the keys it renders. */
const CELLS = [
    {caption: 'CPU', fields: ['cpu.usage:percent', 'cpu.temp:celsius']},
    {caption: 'RAM', fields: ['memory.used:percent']},
    {caption: 'GPU', fields: ['gpu.usage:percent', 'gpu.temp:celsius']},
    {caption: 'NET', fields: ['net.rx:rate', 'net.tx:rate']},
];

/** Dropdown rows, which carry what does not fit in the panel. */
const ROWS = [
    {title: 'Processor', field: 'cpu.usage:percent'},
    {title: 'Processor temperature', field: 'cpu.temp:celsius'},
    {title: 'Memory', field: 'memory.used:percent'},
    {title: 'Swap', field: 'memory.swap:percent'},
    {title: 'Graphics', field: 'gpu.usage:percent'},
    {title: 'Graphics memory', field: 'gpu.memory:percent'},
    {title: 'Graphics temperature', field: 'gpu.temp:celsius'},
    {title: 'Disk read', field: 'disk.read:rate'},
    {title: 'Disk write', field: 'disk.write:rate'},
    {title: 'Disk temperature', field: 'disk.temp:celsius'},
    {title: 'Network in', field: 'net.rx:rate'},
    {title: 'Network out', field: 'net.tx:rate'},
];

const FORMATTERS = {percent, celsius, rate};

/** Captions are dimmed through the actor, because St implements only a subset
 * of CSS and its opacity handling is not dependable across themes. */
const CAPTION_OPACITY = 140;

export const Indicator = GObject.registerClass(
class Indicator extends PanelMenu.Button {
    _init() {
        super._init(0.5, 'rldyour sysinfo', false);

        // Every label that carries a value, keyed by its `path:formatter`
        // descriptor. One flat map means updating is a single pass with no
        // per-tick lookups into the widget tree.
        this._fields = new Map();

        this.add_child(this._buildPanel());
        this._buildMenu();

        this._client = new Client(sample => this._apply(sample), connected => this._setConnected(connected));
    }

    _buildPanel() {
        const panel = new St.BoxLayout({
            styleClass: 'rldyour-panel',
            yAlign: Clutter.ActorAlign.CENTER,
        });

        for (const cell of CELLS) {
            const box = new St.BoxLayout({
                styleClass: 'rldyour-cell',
                yAlign: Clutter.ActorAlign.CENTER,
            });
            box.add_child(new St.Label({
                text: cell.caption,
                styleClass: 'rldyour-caption',
                yAlign: Clutter.ActorAlign.CENTER,
                opacity: CAPTION_OPACITY,
            }));
            for (const field of cell.fields)
                box.add_child(this._valueLabel(field, 'rldyour-value'));
            panel.add_child(box);
        }

        return panel;
    }

    _buildMenu() {
        for (const row of ROWS) {
            const item = new PopupMenu.PopupBaseMenuItem({reactive: false});
            item.add_child(new St.Label({
                text: row.title,
                styleClass: 'rldyour-row-title',
                xExpand: true,
            }));
            item.add_child(this._valueLabel(row.field, 'rldyour-row-value'));
            this.menu.addMenuItem(item);
        }
    }

    /** A label registered for updates, so `_apply` can reach it by descriptor. */
    _valueLabel(field, styleClass) {
        const label = new St.Label({
            text: ABSENT,
            styleClass,
            yAlign: Clutter.ActorAlign.CENTER,
        });

        let labels = this._fields.get(field);
        if (labels === undefined) {
            labels = [];
            this._fields.set(field, labels);
        }
        labels.push(label);

        return label;
    }

    _apply(sample) {
        for (const [field, labels] of this._fields) {
            const [path, formatter] = field.split(':');
            const text = FORMATTERS[formatter](resolve(sample, path));
            for (const label of labels) {
                // Assigning identical text still invalidates the actor and
                // costs a relayout, which at this cadence is the difference
                // between an idle panel and a permanently redrawing one.
                if (label.text !== text)
                    label.text = text;
            }
        }
    }

    _setConnected(connected) {
        if (connected)
            return;

        // Stale numbers are worse than none: a frozen reading looks like a
        // healthy idle system.
        for (const labels of this._fields.values()) {
            for (const label of labels)
                label.text = ABSENT;
        }
    }

    destroy() {
        this._client.stop();
        this._client = null;
        this._fields.clear();
        super.destroy();
    }
});

/** Reads `a.b` out of a decoded sample, tolerating an absent branch. */
function resolve(sample, path) {
    let value = sample;
    for (const key of path.split('.')) {
        if (value === null || typeof value !== 'object')
            return null;
        value = value[key];
    }
    return value;
}
