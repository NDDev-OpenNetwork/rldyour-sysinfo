/* rldyour-sysinfo — preferences
 * Copyright (C) 2026 NDDev OpenNetwork
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Runs in its own process, which is why nothing here touches St, Clutter, Meta
 * or Shell: those libraries exist only inside gnome-shell.
 */

import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

/** Panel toggles, in the order they appear in the panel itself. */
const TOGGLES = [
    {key: 'show-cpu', title: 'Processor', subtitle: 'Busy share of all cores'},
    {key: 'show-memory', title: 'Memory', subtitle: 'Share unavailable to new allocations'},
    {key: 'show-gpu', title: 'Graphics', subtitle: 'Requires an NVIDIA driver'},
    {key: 'show-network', title: 'Network', subtitle: 'Throughput across physical interfaces'},
    {key: 'show-temperatures', title: 'Temperatures', subtitle: 'Shown next to processor and graphics'},
];

export default class SysinfoPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const settings = this.getSettings();
        const page = new Adw.PreferencesPage();

        const reading = new Adw.PreferencesGroup({
            title: 'Readings',
            description: 'The daemon serves the fastest cadence any client asks for.',
        });
        const interval = new Adw.SpinRow({
            title: 'Interval',
            subtitle: 'Seconds between readings',
            adjustment: new Gtk.Adjustment({lower: 1, upper: 60, stepIncrement: 1, pageIncrement: 5}),
        });
        settings.bind('interval', interval, 'value', Gio.SettingsBindFlags.DEFAULT);
        reading.add(interval);
        page.add(reading);

        const panel = new Adw.PreferencesGroup({
            title: 'Panel',
            description: 'What appears in the top bar. The menu always lists everything.',
        });
        for (const toggle of TOGGLES) {
            const row = new Adw.SwitchRow({title: toggle.title, subtitle: toggle.subtitle});
            settings.bind(toggle.key, row, 'active', Gio.SettingsBindFlags.DEFAULT);
            panel.add(row);
        }
        page.add(panel);

        window.add(page);
    }
}
