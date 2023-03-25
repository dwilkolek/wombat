<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";

  let clusterList = [];

  async function clusters() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    clusterList = await invoke("clusters");
  }
</script>

<div>
  <div class="row">
    <button on:click={clusters}> Get Clusters </button>
  </div>
  <ul>
    {#each clusterList as cluster}
      <li>{JSON.stringify(cluster)}</li>
    {/each}
  </ul>
</div>
