<script lang="ts">
    import type { WiFiNetwork } from "../utils/api";

    export let networks: WiFiNetwork[] = [];
    export let isScanning: boolean = false;
</script>

{#if isScanning}
    <div class="w-full p-4 bg-gray-700 rounded-lg mb-4">
        <div class="flex items-center mb-2">
            <svg class="animate-spin h-5 w-5 mr-2" viewBox="0 0 24 24">
                <circle
                    class="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    stroke-width="4"
                    fill="none"
                />
                <path
                    class="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                />
            </svg>
            <span class="text-sm">Scanning for networks...</span>
        </div>
        {#if networks.length > 0}
            <div class="space-y-2">
                <div class="text-sm text-gray-300">
                    Found {networks.length} networks so far
                </div>
                <div class="text-xs text-gray-400">
                    Latest network: {networks[networks.length - 1]?.ssid ||
                        "Unknown"}
                </div>
            </div>
        {/if}
    </div>
{/if}
