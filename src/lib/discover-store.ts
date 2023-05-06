import { writable } from 'svelte/store';
import { execute } from './error-store';

import type { AwsEnv } from './types';
import { userStore } from './user-store';

const createDiscoverStore = () => {
	let favoriteArns: string[] = [];
	let allDiscovered: [string, AwsEnv, string, string][] | undefined = undefined;
	const discovered = writable<[string, AwsEnv, string, string][] | undefined>();
	const refresh = (newSearch: boolean) => {
		discovered.update((old) => {
			if (!newSearch && old == undefined) {
				return old;
			}
			return allDiscovered?.filter((d) => !favoriteArns.includes(d[2]));
		});
	};
	userStore.subscribe((userConfig) => {
		favoriteArns = [...userConfig.tracked_names];
		refresh(false);
	});

	const discover = async (name: string) => {
		const new_options = await execute<[string, AwsEnv, string, string][]>(
			'discover',
			{ name },
			true
		);
		allDiscovered = new_options;
		refresh(true);
	};
	return {
		discover,
		...discovered
	};
};

export const discoverStore = createDiscoverStore();
