<script lang="ts">
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import type { DatabaseCredentials, RdsInstance } from '$lib/types';
	import { isCommandError } from '$lib/utils';
	import { invoke } from '@tauri-apps/api/core';
	import { userStore } from '$lib/stores/user-store';
	import { getRdsSecretDisabledReason } from '$lib/stores/reasons';
	import { featuresStore } from '$lib/stores/feature-store';

	interface Props {
		database: RdsInstance | undefined;
	}

	let { database }: Props = $props();

	let dialogState:
		| { state: 'present' }
		| { state: 'loading' }
		| { state: 'success'; credentials: DatabaseCredentials }
		| { state: 'error'; error: string } = $state({ state: 'present' });

	let disabledReason = getRdsSecretDisabledReason(database);
	let dialog: HTMLDialogElement | undefined = $state();
</script>

{#if database}
	<div class="tooltip tooltip-left" data-tip={$disabledReason ?? 'Search for secret'}>
		<button
			disabled={!!$disabledReason}
			class={$disabledReason ? 'cursor-pointer opacity-30' : 'cursor-pointer'}
			onclick={() => {
				dialogState = { state: 'present' };
				dialog?.show();
			}}
			aria-label={$disabledReason ?? 'Search for secret'}
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				viewBox="0 0 20 20"
				fill="currentColor"
				class="w-4 h-4 text-amber-300"
			>
				<path
					fill-rule="evenodd"
					d="M8 7a5 5 0 113.61 4.804l-1.903 1.903A1 1 0 019 14H8v1a1 1 0 01-1 1H6v1a1 1 0 01-1 1H3a1 1 0 01-1-1v-2a1 1 0 01.293-.707L8.196 8.39A5.002 5.002 0 018 7zm5-3a.75.75 0 000 1.5A1.5 1.5 0 0114.5 7 .75.75 0 0016 7a3 3 0 00-3-3z"
					clip-rule="evenodd"
				/>
			</svg>
		</button>
	</div>

	<dialog bind:this={dialog} class="modal">
		<div class="modal-box">
			<button
				class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
				onclick={() => dialog?.close()}>✕</button
			>
			<h3 class="text-lg font-bold">
				Fetch credentials to <span class="text-warning">{database.identifier}</span>
			</h3>
			{#if dialogState.state == 'present'}
				<p class="py-4">
					Are you alone and not sharing screen?<br />
					Access to credentials is recorded.
				</p>
				<div class="modal-action">
					<button
						class="btn btn-secondary"
						onclick={() => {
							dialog?.close();
						}}
					>
						Abort
					</button>
					<button
						class="btn btn-primary"
						data-umami-event="rds_credentials_get"
						data-umami-event-uid={$userStore.id}
						onclick={async () => {
							try {
								dialogState = { state: 'loading' };
								const credentials = await invoke<DatabaseCredentials>('credentials', {
									db: database
								});
								dialogState = { state: 'success', credentials };
							} catch (e: unknown) {
								dialogState = {
									state: 'error',
									error:
										e instanceof Error || isCommandError(e)
											? e.message
											: typeof e === 'string'
												? e
												: 'Unknown error occurred'
								};
							}
						}}
					>
						Get me that secret!
					</button>
				</div>
			{/if}
			{#if dialogState.state === 'loading'}
				<p class="py-4">
					<span class="loading loading-spinner text-accent"></span>
				</p>
				<div class="modal-action">
					<button
						class="btn btn-primary"
						onclick={() => {
							dialog?.close();
						}}
					>
						Abort
					</button>
				</div>
			{/if}
			{#if dialogState.state === 'success'}
				{@const secret = dialogState.credentials}
				<div class="py-4 flex flex-col gap-2 text-sm">
					<div class="flex flex-row items-center gap-1">
						<span class="self-start">ARN:</span>
						<span class="font-mono text-warning truncate">
							{secret.arn.split(':').slice(0, 5).join(':')}:<br />
							{secret.arn.split(':').slice(5).join(':')}
						</span>
					</div>
					<div class="flex flex-row items-center gap-1">
						<span>DB name:</span>
						<span class="font-mono text-warning">{secret.dbname}</span>
					</div>
					<div class="flex flex-row items-center gap-1">
						<span>Username:</span>
						<span class="font-mono text-warning">{secret.username}</span>
					</div>
					<div class="flex flex-row items-center gap-1">
						<span>Password:</span>
						<span class="font-mono text-warning">{secret.password}</span>
					</div>
					<div class="flex flex-row items-center gap-1">
						<span>Auto rotated:</span>
						<span class="font-mono text-warning">{secret.auto_rotated ? 'Yes' : 'No'}</span>
					</div>
				</div>
				<div class="modal-action">
					<button
						class="btn btn-secondary"
						data-umami-event="rds_credentials_copy"
						data-umami-event-uid={$userStore.id}
						onclick={() => {
							writeText(secret.password);
						}}
					>
						Copy password
					</button>
					<button
						class="btn btn-primary"
						onclick={() => {
							dialog?.close();
						}}
					>
						Close
					</button>
				</div>
			{/if}
			{#if dialogState.state === 'error'}
				<p class="py-4">
					Secret not found 🥲<br />
					{#if $featuresStore.debug}
						<span class="text-sm text-error">
							Reason: {dialogState.error}
						</span>
					{/if}
				</p>

				<div class="modal-action">
					<button
						class="btn btn-primary"
						onclick={() => {
							dialog?.close();
						}}
					>
						Close
					</button>
				</div>
			{/if}
		</div>
	</dialog>
{/if}
