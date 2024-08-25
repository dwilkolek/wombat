import type { WombatAwsProfile, InfraProfile } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';
import { derived, writable } from 'svelte/store';
import { userStore } from './user-store';
import { envImportance, envStore } from './env-store';

const createAvailableProfilesStore = () => {
	const wombatAwsProfiles = writable<WombatAwsProfile[]>([]);

	invoke<WombatAwsProfile[]>('wombat_aws_profiles').then((resp) => {
		wombatAwsProfiles.set(resp);
	});

	const refresh = () => {
		invoke<WombatAwsProfile[]>('wombat_aws_profiles').then((resp) => {
			wombatAwsProfiles.set(resp);
		});
	};

	return {
		refresh,
		wombatAwsProfiles
	};
};

export const availableProfilesStore = createAvailableProfilesStore();

export const wombatProfileStore = derived(
	[userStore, availableProfilesStore.wombatAwsProfiles],
	(stores) => {
		const wombatProfile = stores[1].find((wp) => wp.name == stores[0].last_used_profile);
		const infraProfiles: InfraProfile[] = [];
		if (wombatProfile?.sso_profiles) {
			infraProfiles.push(
				...Object.values(wombatProfile.sso_profiles)
					.map((sso) => sso.infra_profiles)
					.flat()
			);
		}
		const ssoProfiles = wombatProfile?.sso_profiles
			? Object.values(wombatProfile.sso_profiles)
			: [];
		const environments = ssoProfiles
			.map((sso) => sso.env)
			.sort((a, b) => envImportance[a] - envImportance[b]);

		envStore.update((oldEnv) =>
			environments.includes(oldEnv) ? oldEnv : (environments[0] ?? oldEnv)
		);

		return {
			wombatProfile,
			ssoProfiles,
			infraProfiles,
			environments
		};
	}
);
