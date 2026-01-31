<script lang="ts">
	let {
		items = [],
		selected = $bindable([]),
		placeholder = 'Select Items...'
	}: { items: string[]; selected: string[]; placeholder?: string } = $props();

	let open = $state(false);
	let inputValue = $state('');
	let inputElement: HTMLInputElement | undefined = $state();
	let filteredItems = $derived(
		items.filter((i) => i.toLowerCase().includes(inputValue.toLowerCase()))
	);

	const toggle = () => {
		open = !open;
		if (!open) {
			inputValue = '';
		} else {
			// Svelte 5 tick or just timeout to focus
			setTimeout(() => inputElement?.focus(), 0);
		}
	};

	const select = (item: string) => {
		if (selected.includes(item)) {
			// Remove
			selected = selected.filter((i) => i !== item);
		} else {
			// Add
			selected = [...selected, item];
		}
		if (inputElement) {
			inputElement.focus();
		}
	};

	const remove = (item: string) => {
		selected = selected.filter((i) => i !== item);
	};
</script>

<div class="w-full flex flex-col items-center mx-auto">
	<div class="w-full">
		<div class="flex flex-col items-center relative cursor-pointer">
			<div class="w-full">
				<div
					class="min-h-8 flex flex-row justify-center items-center p-1 form-control border border-b-2 rounded-sm border-white/20 rounded-btn bg-base-100"
				>
					<div class="flex flex-auto flex-wrap items-center gap-1 shrink">
						{#each selected as s}
							<div class="badge badge-info text-xs">
								{s}
								<button
									onclick={() => remove(s)}
									aria-label={'Remove ' + s}
									type="button"
									class="ml-1"
								>
									<svg
										xmlns="http://www.w3.org/2000/svg"
										fill="none"
										viewBox="0 0 24 24"
										class="inline-block w-4 h-4 stroke-current"
										><path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M6 18L18 6M6 6l12 12"
										></path></svg
									>
								</button>
							</div>
						{/each}

						<!-- Helper to toggle open/close when clicking empty space -->
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div class="grow h-full min-h-[24px]" onclick={toggle}>&nbsp;</div>
					</div>

					<button class="z-50 outline-none focus:outline-none ml-2" type="button" onclick={toggle}>
						{#if open}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								viewBox="0 0 20 20"
								fill="currentColor"
								class="w-5 h-5"
							>
								<path
									fill-rule="evenodd"
									d="M14.77 12.79a.75.75 0 01-1.06-.02L10 8.832 6.29 12.77a.75.75 0 11-1.08-1.04l4.25-4.5a.75.75 0 011.08 0l4.25 4.5a.75.75 0 01-.02 1.06z"
									clip-rule="evenodd"
								/>
							</svg>
						{:else}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								viewBox="0 0 20 20"
								fill="currentColor"
								class="w-5 h-5"
							>
								<path
									fill-rule="evenodd"
									d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z"
									clip-rule="evenodd"
								/>
							</svg>
						{/if}
					</button>
				</div>
			</div>
			{#if open}
				<div
					class="absolute shadow top-full mt-1
					bg-base-200 w-full left-0 rounded-md overflow-y-auto z-50 border border-base-300"
				>
					<div class="flex flex-col w-full base-300">
						<div class="m-2">
							<input
								autocomplete="off"
								autocorrect="off"
								autocapitalize="off"
								spellcheck="false"
								type="text"
								{placeholder}
								class="input input-sm input-bordered input-accent w-full"
								bind:value={inputValue}
								bind:this={inputElement}
							/>
						</div>
						<div class="overflow-auto max-h-[200px]">
							{#each filteredItems as item}
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class="cursor-pointer w-full hover:bg-base-300"
									onclick={() => {
										select(item);
										inputValue = '';
									}}
								>
									<div class="flex w-full items-center p-2 border-transparent border-l-2 relative">
										<div class="w-full items-center flex">
											<div class="mx-2 leading-6 flex items-center gap-2">
												{#if selected.includes(item)}
													<svg
														xmlns="http://www.w3.org/2000/svg"
														viewBox="0 0 20 20"
														fill="currentColor"
														class="w-4 h-4 text-primary"
													>
														<path
															fill-rule="evenodd"
															d="M16.704 4.153a.75.75 0 01.143 1.052l-8 10.5a.75.75 0 01-1.127.075l-4.5-4.5a.75.75 0 011.06-1.06l3.894 3.893 7.48-9.817a.75.75 0 011.05-.143z"
															clip-rule="evenodd"
														/>
													</svg>
												{:else}
													<div class="w-4 h-4"></div>
												{/if}
												{item}
											</div>
										</div>
									</div>
								</div>
							{/each}
							{#if filteredItems.length === 0}
								<div class="p-2 text-center opacity-50 text-sm">No matches found</div>
							{/if}
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="w-screen h-screen bottom-0 left-0 fixed bg-transparent z-40" onclick={toggle}></div>
{/if}
