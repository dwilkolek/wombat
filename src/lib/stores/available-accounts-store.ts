import { AwsEnv, type WombatAwsAccount, type InfraProfile } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';
import { derived, writable } from 'svelte/store';
import { userStore } from './user-store';
import { envImportance, envStore } from './env-store';

const createAvailableAccountsStore = () => {
	const wombatAwsAccounts = writable<WombatAwsAccount[]>([]);

	invoke<WombatAwsAccount[]>('wombat_aws_profiles').then((resp) => {
		wombatAwsAccounts.set(resp.sort((a, b) => a.name.localeCompare(b.name)));
	});

	const refresh = async () => {
		return invoke<WombatAwsAccount[]>('wombat_aws_profiles').then((resp) => {
			wombatAwsAccounts.set(resp.sort((a, b) => a.name.localeCompare(b.name)));
		});
	};

	return {
		refresh,
		wombatAwsAccounts
	};
};

export const availableAccountsStore = createAvailableAccountsStore();

export const wombatAccountStore = derived(
	[userStore, availableAccountsStore.wombatAwsAccounts],
	(stores) => {
		const wombatAccount = stores[1].find((wp) => wp.id == stores[0].last_used_profile);
		const infraProfiles: InfraProfile[] = [];
		if (wombatAccount?.sso_profiles) {
			infraProfiles.push(
				...Object.values(wombatAccount.sso_profiles)
					.flat()
					.map((sso) => sso?.infra_profiles ?? [])
					.flat()
			);
		}
		const ssoProfiles = wombatAccount?.sso_profiles
			? Object.values(wombatAccount.sso_profiles).flat()
			: [];
		const environments = [
			...new Set(ssoProfiles.map((sso) => sso?.env).filter((env): env is AwsEnv => !!env))
		].sort((a, b) => envImportance[a] - envImportance[b]);

		envStore.update((oldEnv) =>
			environments.includes(oldEnv) ? oldEnv : (environments[0] ?? oldEnv)
		);

		return {
			wombatAccount,
			ssoProfiles,
			infraProfiles,
			environments
		};
	}
);
