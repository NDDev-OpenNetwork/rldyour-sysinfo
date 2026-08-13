/* rldyour-sysinfo — companion daemon client
 * Copyright (C) 2026 NDDev OpenNetwork
 * SPDX-License-Identifier: AGPL-3.0-or-later
 */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

const SOCKET_NAME = 'rldyour-sysinfo.sock';
const RECONNECT_SECONDS = 5;
/** Refuse a line long enough to mean the peer is not our daemon. */
const MAX_LINE = 4096;

/**
 * Reads newline-delimited samples from the daemon socket.
 *
 * The cadence belongs entirely to the daemon: this client never runs a timer
 * while it is connected, it only waits on the next line. The single timer it
 * can own is the reconnect backoff, which exists because the daemon is allowed
 * to exit when nobody is watching.
 */
export class Client {
    /**
     * @param {(sample: object) => void} onSample called for each decoded line
     * @param {(connected: boolean) => void} onState called when the link changes
     */
    constructor(onSample, onState) {
        this._onSample = onSample;
        this._onState = onState;
        this._cancellable = new Gio.Cancellable();
        this._connection = null;
        this._stream = null;
        this._reconnectId = 0;
        this._connect();
    }

    _connect() {
        const path = GLib.build_filenamev([GLib.get_user_runtime_dir(), SOCKET_NAME]);
        const client = new Gio.SocketClient();

        client.connect_async(new Gio.UnixSocketAddress({path}), this._cancellable, (source, result) => {
            let connection;
            try {
                connection = source.connect_finish(result);
            } catch (error) {
                if (!error.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.CANCELLED))
                    this._retry();
                return;
            }

            this._connection = connection;
            this._stream = new Gio.DataInputStream({baseStream: connection.get_input_stream()});
            this._onState(true);
            this._read();
        });
    }

    _read() {
        this._stream.read_line_async(GLib.PRIORITY_DEFAULT, this._cancellable, (source, result) => {
            let line;
            try {
                [line] = source.read_line_finish_utf8(result);
            } catch (error) {
                if (!error.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.CANCELLED))
                    this._retry();
                return;
            }

            // A null line is a clean close, which happens whenever the daemon
            // decides nobody needed it any more.
            if (line === null) {
                this._retry();
                return;
            }

            if (line.length <= MAX_LINE)
                this._decode(line);

            // The consumer is allowed to stop us from inside the callback
            // above, which is exactly what the indicator does when the shell
            // disables the extension. That tears the stream down, so there is
            // nothing left to continue reading from.
            if (this._stream !== null)
                this._read();
        });
    }

    _decode(line) {
        let sample;
        try {
            sample = JSON.parse(line);
        } catch {
            // A malformed line means a version skew or a foreign peer. Skipping
            // it keeps the indicator on its last good reading.
            return;
        }
        this._onSample(sample);
    }

    _retry() {
        this._closeConnection();
        this._onState(false);

        if (this._reconnectId)
            GLib.Source.remove(this._reconnectId);
        this._reconnectId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, RECONNECT_SECONDS, () => {
            this._reconnectId = 0;
            this._connect();
            return GLib.SOURCE_REMOVE;
        });
    }

    _closeConnection() {
        if (this._connection)
            this._connection.close_async(GLib.PRIORITY_DEFAULT, null, null);
        this._connection = null;
        this._stream = null;
    }

    /** Releases the socket and the reconnect timer. The client is done after this. */
    stop() {
        if (this._reconnectId)
            GLib.Source.remove(this._reconnectId);
        this._reconnectId = 0;

        this._cancellable.cancel();
        this._closeConnection();
    }
}
