// import { execute } from '$lib/error-store';
// import type { DbInstance, ServiceDetails } from '$lib/types';
// import { listen } from '@tauri-apps/api/event';
// import { readable, writable } from 'svelte/store';
// export type HomeEntry = {
// 	tracked_name: string;
// 	services: Map<string, ServiceDetails[]>;
// 	dbs: DbInstance[];
// };
// type HomePage = {
// 	entries: HomeEntry[];
// };

// const createHome = () => {
// 	const entries = writable<HomeEntry[]>([]);
// 	listen('cache-refreshed', () => {
// 		refresh();
// 	});
// 	listen<HomePage>('new-home-cache', (e) => {
// 		console.log('new-home-cache', e);
// 		handleNewHome(e.payload);
// 	});
// 	const refresh = (tracking = false) => {
// 		execute<HomePage>('home', undefined, tracking).then((home) => {
// 			handleNewHome(home);
// 		});
// 	};

// 	const handleNewHome = (home: HomePage) => {
// 		console.log('handleNewHome', home);
// 		entries.set(home.entries);
// 	};
// 	const entriesReadable = readable([] as HomeEntry[], (set) => {
// 		set([]);
// 		entries.subscribe((s) => {
// 			set(s);
// 		});
// 	});

// 	const initialized = false;
// 	return {
// 		subscribe: entriesReadable.subscribe,
// 		refresh,
// 		init: () => {
// 			if (!initialized) {
// 				refresh(true);
// 			}
// 		}
// 	};
// };

// export const homeStore = createHome();
