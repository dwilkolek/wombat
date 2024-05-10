import type { AwsEnv } from '$lib/types';
import { invoke } from '@tauri-apps/api';
import { writable } from 'svelte/store';
type InfraProfile = [string, AwsEnv];
const createAvailableProfilesStore = () => {
	const infraProfiles = writable<InfraProfile[]>([]);
	const ssoProfiles = writable<string[]>([]);

	invoke<InfraProfile[]>('available_infra_profiles').then((resp) => {
		infraProfiles.set(resp);
	});
	invoke<string[]>('available_sso_profiles').then((resp) => {
		ssoProfiles.set(resp);
	});
	const refresh = () => {
		invoke<InfraProfile[]>('available_infra_profiles').then((resp) => {
			infraProfiles.set(resp);
		});
		invoke<string[]>('available_sso_profiles').then((resp) => {
			ssoProfiles.set(resp);
		});
	};

	return {
		refresh,
		infraProfiles,
		ssoProfiles
	};
};

export const availableProfilesStore = createAvailableProfilesStore();
