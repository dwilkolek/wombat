import type { AwsEnv } from '$lib/types';
import { invoke } from '@tauri-apps/api';
import { writable } from 'svelte/store';
type SsoProfile = {
	profile_name: string;
	region?: string;
	infra_profiles: InfraProfile[];
};

type InfraProfile = {
	source_profile: string;
	profile_name: string;
	region?: string;
	app: string;
	env: AwsEnv;
};
const createAvailableProfilesStore = () => {
	const infraProfiles = writable<InfraProfile[]>([]);
	const ssoProfiles = writable<SsoProfile[]>([]);

	invoke<InfraProfile[]>('available_infra_profiles').then((resp) => {
		infraProfiles.set(resp);
	});
	invoke<SsoProfile[]>('available_sso_profiles').then((resp) => {
		console.log(resp);
		ssoProfiles.set(resp);
	});
	const refresh = () => {
		invoke<InfraProfile[]>('available_infra_profiles').then((resp) => {
			infraProfiles.set(resp);
		});
		invoke<SsoProfile[]>('available_sso_profiles').then((resp) => {
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
