<script lang="ts">
	import { writeText } from '@tauri-apps/api/clipboard';
	import type { DatabaseCredentials, RdsInstance } from '$lib/types';
	import { ask, message } from '@tauri-apps/api/dialog';
	import { invoke } from '@tauri-apps/api/tauri';
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';
	import { featuresStore } from '$lib/stores/feature-store';
	import { userStore } from '$lib/stores/user-store';
	export let database: RdsInstance | undefined;

	const credentialsHandler = async () => {
		let answer = await ask(
			'Are you alone and not sharing screen?\nAccess to credentials is recorded.\nRequires access to Secret Manager.',
			{
				title: 'Are you alone?',
				okLabel: "It's safe!",
				cancelLabel: 'No',
				type: 'warning'
			}
		);
		if (answer) {
			try {
				const credentials = await invoke<DatabaseCredentials>('credentials', { db: database });
				const copyToClipboard = await ask(
					`Database name: ${credentials.dbname}\nUser: ${credentials.username}\nPassword: ${credentials.password}\nRotated: ${credentials.auto_rotated}\nWhen 'Rotated'=false then User & Database name might be wrong.`,
					{ title: 'Credentials', okLabel: 'Copy password to clipboard', cancelLabel: 'K THX BYE' }
				);
				if (copyToClipboard) {
					await writeText(credentials.password);
				}
			} catch (e) {
				message(
					`Credentials not found for ${database?.name}.\n Did you configure profile for ${database?.name} database?`,
					{ title: 'Ooops!', type: 'error' }
				);
			}
		}
	};
</script>

{#if database}
	{#if $featuresStore.devWay || $wombatProfileStore.infraProfiles.some(({ app, env }) => app == database?.normalized_name && env == database?.env)}
		<div class="tooltip tooltip-left" data-tip="Search for secret">
			<button
				on:click={credentialsHandler}
				data-umami-event="rds_credentials_get"
				data-umami-event-uid={$userStore.id}
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
	{:else}
		<div class="tooltip tooltip-left" data-tip={`Missing aws profile: ${database.normalized_name}`}>
			<button class="opacity-30" disabled>
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
	{/if}
{/if}
