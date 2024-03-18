import { invoke } from '@tauri-apps/api';
import { writable } from 'svelte/store';

const createAvailableProfilesStore = () => {
	const infraProfiles = writable<string[]>([]);
    const ssoProfiles = writable<string[]>([]);

	invoke<string[]>('available_infra_profiles').then((resp) => {
		infraProfiles.set(resp);
	});
    invoke<string[]>('available_sso_profiles').then((resp) => {
		ssoProfiles.set(resp);
	});
	const refresh = () => {
		invoke<string[]>('available_infra_profiles').then((resp) => {
			infraProfiles.set(resp);
		});
        invoke<string[]>('available_sso_profiles').then((resp) => {
			ssoProfiles.set(resp);
		});
    }
   
	
	return {
		refresh,
		infraProfiles,
        ssoProfiles
	};
};

export const availableProfilesStore = createAvailableProfilesStore();
