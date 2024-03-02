import { invoke } from '@tauri-apps/api/core';
import type { InvokeArgs } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

export const error = writable<string | undefined>();
export const loading = writable<string | undefined>();
export async function execute<T>(
	cmd: string,
	args: InvokeArgs | undefined = undefined,
	trackLoading = false
): Promise<T> {
	try {
		error.set(undefined);
		if (trackLoading) {
			loading.set(cmd);
		}
		error.set(undefined);
		return await invoke(cmd, args);
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
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
