import { writable } from 'svelte/store';
import { type BrowserExtensionStatus } from '../types';
import { invoke } from '@tauri-apps/api/tauri';
let timeout: number | undefined = undefined;
const createExtensionStatus = () => {
	const state = writable<BrowserExtensionStatus>({ connected: false, cookie_health: {} });
	const scheduleNext = (time: number) =>
		setTimeout(() => {
			console.log('checking browser status');
			clearTimeout(timeout);
			invoke<BrowserExtensionStatus>('browser_extension_health').then((res) => {
				state.set(res);
				if (res.connected) {
					scheduleNext(10000);
				} else {
					scheduleNext(1000);
				}
			});
		}, time);
	timeout = scheduleNext(1000);
	return state;
};

export const browserExtensionStatus = createExtensionStatus();
