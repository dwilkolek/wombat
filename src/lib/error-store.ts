import { invoke } from '@tauri-apps/api';
import type { InvokeArgs } from '@tauri-apps/api/tauri';
import { writable } from 'svelte/store';

export const error = writable<string | undefined>();
export const loading = writable<String | undefined>();
export async function execute<T>(
	cmd: string,
	args: InvokeArgs | undefined = undefined,
	trackLoading = false
): Promise<T> {
	try {
		error.set(undefined);
		console.log('Invoking ', cmd, trackLoading);
		if (trackLoading) {
			loading.set(cmd);
		}
		error.set(undefined);
		return await invoke(cmd, args);
	} catch (e: any) {
		console.error('Error occured', cmd, e);
		error.set(`${e.message ?? e}`);
		throw e;
	} finally {
		if (trackLoading) {
			loading.set(undefined);
		}
	}
}
