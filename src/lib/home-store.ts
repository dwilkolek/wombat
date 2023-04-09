import { execute } from '$lib/error-store';
import type { AwsEnv, DbInstance, ServiceDetails } from '$lib/types';
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
	const refresh = (tracking = false) => {
		execute<HomePage>('home', undefined, tracking).then((home) => {
			const newEntries: HomeEntries = {};
			const newArnList: string[] = [];
			home.services.forEach((v) => {
				if (!newEntries[v.name]) {
					newEntries[v.name] = {};
				}
				if (!newEntries[v.name][v.env]) {
					newEntries[v.name][v.env] = {};
				}
				newArnList.push(v.arn);
				newEntries[v.name][v.env].service = v;
			});

			home.databases.forEach((v) => {
				if (!newEntries[v.name]) {
					newEntries[v.name] = {};
				}
				if (!newEntries[v.name][v.environment_tag.toUpperCase()]) {
					newEntries[v.name][v.environment_tag.toUpperCase()] = {};
				}
				newArnList.push(v.arn);
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

	const initialized = false;
	return {
		subscribe: entriesReadable.subscribe,
		refresh,
		init: () => {
			if (!initialized) {
				refresh(true);
			}
		}
	};
};

export const homeStore = createHome();
