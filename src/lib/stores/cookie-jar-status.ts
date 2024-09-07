import { writable } from 'svelte/store';
import { BrowserExtensionState, type CookieJarStatus } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { browserExtensionStatus } from './browser-extension-status';
let timeout: number | undefined = undefined;

const createCookieJarStatus = () => {
	const state = writable<CookieJarStatus>({ cookieHealth: {} });
	let browserExtensionConnected = false;
	const scheduleNext = (time: number) =>
		setTimeout(() => {
			console.log('checking cookie jar status');
			clearTimeout(timeout);

			browserExtensionStatus.subscribe((browserExtension) => {
				browserExtensionConnected = browserExtension.state != BrowserExtensionState.Disconnected;
			});
			invoke<CookieJarStatus>('cookie_jar_status').then((res) => {
				state.set(res);
				if (browserExtensionConnected) {
					scheduleNext(10000);
				} else {
					scheduleNext(1000);
				}
			});
		}, time);
	timeout = scheduleNext(1000);
	return state;
};

export const cookieJar = createCookieJarStatus();
