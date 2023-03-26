<script lang="ts">
  import Menu from "./lib/Menu.svelte";
  import { state } from "./lib/store";
  let records = state.records;
</script>

<main class="min-h-screen">
  <Menu />

  <div class="mx-auto">
    <div class="overflow-x-auto">
      <table class="table w-full">
        <!-- head -->
        <thead>
          <tr>
            <th />
            <th>Service name</th>
            <th>DBs</th>
          </tr>
        </thead>
        <tbody>
          {#each $records as record, i}
            <tr>
              <th>{i}</th>
              <td>{record.service}</td>
              <td>
                {#each record.dbs as db}
                  <ul>
                    <li>{db.db_name}</li>
                  </ul>
                {/each}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</main>

<style>
</style>
