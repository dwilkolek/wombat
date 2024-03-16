<script lang="ts">
	import { writable } from 'svelte/store';
	import '$lib/styles.css';
    import Icon from '$lib/images/128x128.png';
	import { goto } from '$app/navigation';
	import { userStore } from '$lib/stores/user-store';

	let profiles = writable<string[]>([]);
	let activeProfile = writable<string | null>(null);
	let dialog: HTMLDialogElement;
	let newProfile = '';
	$: linkClasses = (profile: string | null) => {
			return $activeProfile === profile ? 'text-accent' : '';
		}
    let { subscribe, login } = userStore;
    
	
</script>

<div class="flex gap-1">
	<nav class="flex flex-col gap-2 bg-base-200 min-h-screen">
		<ul class="menu bg-base-200 w-56 rounded-box">
			<li>
				<button class={`flex gap-2 ${linkClasses(null)}`} on:click={() => {
                    activeProfile.set(null);
                    goto(`/`, { replaceState: true });
                }}
					>
					<img class="h-5 w-5" alt="wombat" src={Icon} />
                    <span>Home</span></button
				>
			</li>
			{#each $profiles as profile}
				<li>
					<button
						class={`flex gap-2 ${linkClasses(profile)}`}
						on:click={async () => {
                            
                            await login(profile);
                            activeProfile.set(profile);
                            goto(`/teams/${profile}`, { replaceState: true });
                        }}
					>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							fill="none"
							viewBox="0 0 24 24"
							stroke-width="1.5"
							stroke="currentColor"
							class="w-5 h-5"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								d="M17.982 18.725A7.488 7.488 0 0 0 12 15.75a7.488 7.488 0 0 0-5.982 2.975m11.963 0a9 9 0 1 0-11.963 0m11.963 0A8.966 8.966 0 0 1 12 21a8.966 8.966 0 0 1-5.982-2.275M15 9.75a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"
							/>
						</svg>
						<span>{profile}</span>
					</button>
				</li>
			{/each}
			<li>
				<button class="flex gap-2" on:click={() => dialog.showModal()}
					><svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-5 h-5"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M18 7.5v3m0 0v3m0-3h3m-3 0h-3m-2.25-4.125a3.375 3.375 0 1 1-6.75 0 3.375 3.375 0 0 1 6.75 0ZM3 19.235v-.11a6.375 6.375 0 0 1 12.75 0v.109A12.318 12.318 0 0 1 9.374 21c-2.331 0-4.512-.645-6.374-1.766Z"
						/>
					</svg>
					<span>Add profile</span>
				</button>
			</li>
		</ul>
	</nav>
	<main class="grow">
		<slot />
	</main>
</div>
<dialog class="modal" bind:this={dialog}>
	<div class="modal-box">
		<h3 class="font-bold text-lg">Add profile</h3>

		<div class="modal-action">
			<form method="dialog">
				<input
					type="text"
					placeholder="Profile name"
					class="input input-bordered"
					bind:value={newProfile}
				/>
				<button
					class="btn"
					on:click|preventDefault={() => {
						profiles.update((p) => [...p, newProfile]);
						dialog.close();
						newProfile = '';
					}}>Submit</button
				>
				<button
					class="btn"
					on:click|preventDefault={() => {
						dialog.close();
						newProfile = '';
					}}>Close</button
				>
			</form>
		</div>
	</div>
</dialog>
