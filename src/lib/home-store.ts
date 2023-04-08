import { execute } from '$lib/error-store';
import type { DbInstance, ServiceDetails } from '$lib/types';
import { readable, writable } from 'svelte/store';
type HomePage = {
	services: ServiceDetails[];
	databases: DbInstance[];
};
type HomeEntries = {
	[name: string]: {
		[env: string]: {
			db?: DbInstance | undefined;
			service?: ServiceDetails | undefined;
		};
	};
};

const createHome = () => {
	const entries = writable<HomeEntries>({});

	const refresh = (tracking: boolean = false) => {
		execute<HomePage>('home', undefined, tracking).then((home) => {
			let newEntries: HomeEntries = {};
			home.services.forEach((v) => {
				if (!newEntries[v.name]) {
					newEntries[v.name] = {};
				}
				if (!newEntries[v.name][v.env]) {
					newEntries[v.name][v.env] = {};
				}
				newEntries[v.name][v.env].service = v;
			});

			home.databases.forEach((v) => {
				if (!newEntries[v.name]) {
					newEntries[v.name] = {};
				}
				if (!newEntries[v.name][v.environment_tag.toUpperCase()]) {
					newEntries[v.name][v.environment_tag.toUpperCase()] = {};
				}
				newEntries[v.name][v.environment_tag.toUpperCase()].db = v;
			});
			entries.set(newEntries);
		});
	};
	const entriesReadable = readable({} as HomeEntries, (set) => {
		set({});
		entries.subscribe((s) => {
			set(s);
		});
	});

	const discover = async (name: string) => {
		await execute('discover', { name }, true);
		refresh(true);
	};
	let initialized = false;
	return {
		subscribe: entriesReadable.subscribe,
		refresh,
		discover,
		init: () => {
			if (!initialized) {
				refresh(true);
			}
		}
	};
};

export const homeStore = createHome();
