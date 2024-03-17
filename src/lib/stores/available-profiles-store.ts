import { invoke } from '@tauri-apps/api';
import { writable } from 'svelte/store';

const createAvailableProfilesStore = () => {
	const { subscribe, set, update } = writable<string[]>([]);
	invoke<string[]>('available_profiles').then((resp) => {
		set(resp);
	});
	const refresh = () => {
		invoke<string[]>('available_profiles').then((resp) => {
			set(resp);
		});
	};

	return {
		refresh,
		subscribe,
		set,
		update
	};
};

export const availableProfilesStore = createAvailableProfilesStore();
