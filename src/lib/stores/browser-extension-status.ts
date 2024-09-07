import { writable } from 'svelte/store';
import { BrowserExtensionState, type BrowserExtensionStatus } from '../types';
import { invoke } from '@tauri-apps/api/core';
let timeout: number | undefined = undefined;
const createExtensionStatus = () => {
	const state = writable<BrowserExtensionStatus>({
		state: BrowserExtensionState.Disconnected,
		version: undefined
	});
	const scheduleNext = (time: number) =>
		setTimeout(() => {
			console.log('checking browser status');
			clearTimeout(timeout);
			invoke<BrowserExtensionStatus>('browser_extension_health').then((res) => {
				state.set(res);
				if (BrowserExtensionState.Disconnected != res.state) {
					scheduleNext(10000);
				} else {
					scheduleNext(2000);
				}
			});
		}, time);
	timeout = scheduleNext(2000);
	return state;
};

export const browserExtensionStatus = createExtensionStatus();
