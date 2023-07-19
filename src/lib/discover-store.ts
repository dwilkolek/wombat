// import { writable } from 'svelte/store';
// import { execute } from './error-store';

// import type { AwsEnv } from './types';
// import { userStore } from './user-store';
// import type { HomeEntry } from './home-store';

// const createDiscoverStore = () => {
// 	let favorite_names: string[] = [];
// 	let allDiscovered: HomeEntry[] | undefined = undefined;
// 	const discovered = writable<HomeEntry[] | undefined>();
// 	const refresh = (newSearch: boolean) => {
// 		discovered.update((old) => {
// 			if (!newSearch && old == undefined) {
// 				return old;
// 			}
// 			return allDiscovered?.filter((d) => !favorite_names.includes(d.tracked_name));
// 		});
// 	};
// 	userStore.subscribe((userConfig) => {
// 		favorite_names = [...userConfig.tracked_names];
// 		refresh(false);
// 	});

// 	const discover = async (name: string) => {
// 		const new_options = await execute<HomeEntry[]>(
// 			'discover',
// 			{ name },
// 			true
// 		);
// 		allDiscovered = new_options;
// 		refresh(true);
// 	};
// 	return {
// 		discover,
// 		...discovered
// 	};
// };

// export const discoverStore = createDiscoverStore();
