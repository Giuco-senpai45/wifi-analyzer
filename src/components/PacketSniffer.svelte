<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import {
        startPacketCapture,
        stopPacketCapture,
        listDevices,
        getLatestPackets
    } from "../utils/api";
    import type { PacketInfo } from "../utils/api";

    let allPackets: PacketInfo[] = [];
    let displayedPackets: PacketInfo[] = [];
    let selectedPacket: PacketInfo | null = null;
    let interfaceName = "";
    let isCapturing = false;
    let error: string | null = null;
    let availableInterfaces: string[] = [];
    let updateInterval: number;
    let currentPage = 1;
    const packetsPerPage = 10;

    // Fetch available interfaces on mount
    onMount(async () => {
        try {
            availableInterfaces = await listDevices();
            if (availableInterfaces.length > 0) {
                interfaceName = availableInterfaces[0];
            }
        } catch (err) {
            console.error("Failed to initialize PacketSniffer:", err);
            error = "Failed to initialize. Please check your connection and try again.";
        }
    });

    // Cleanup on component destroy
    onDestroy(() => {
        if (isCapturing) {
            stopCapture();
        }
        if (updateInterval) {
            clearInterval(updateInterval);
        }
    });

    // Start capturing packets
    async function startCapture(): Promise<void> {
        try {
            await startPacketCapture(interfaceName);
            isCapturing = true;
            error = null;
            console.log("Packet capture started");
            updateInterval = setInterval(fetchLatestPackets, 3000);
        } catch (err) {
            console.error("Failed to start packet capture:", err);
            error = "Failed to start packet capture. Please check your permissions and try again.";
        }
    }

    // Stop capturing packets
    async function stopCapture(): Promise<void> {
        try {
            await stopPacketCapture();
            isCapturing = false;
            error = null;
            console.log("Packet capture stopped");
            if (updateInterval) {
                clearInterval(updateInterval);
            }
        } catch (err) {
            console.error("Failed to stop packet capture:", err);
            error = "Failed to stop packet capture. The capture may have already been stopped.";
        }
    }

    // Fetch the latest packets from the backend
    async function fetchLatestPackets(): Promise<void> {
        try {
            const newPackets = await getLatestPackets();
            if (newPackets.length > 0) {
                allPackets = [...allPackets, ...newPackets].sort((a, b) => b.timestamp - a.timestamp);
                updateDisplayedPackets();
            }
        } catch (err) {
            console.error("Failed to fetch latest packets:", err);
        }
    }

    // Update the currently displayed packets
    function updateDisplayedPackets(): void {
        const startIndex = (currentPage - 1) * packetsPerPage;
        displayedPackets = allPackets.slice(startIndex, startIndex + packetsPerPage);
    }

    // Select a specific packet
    function selectPacket(packet: PacketInfo): void {
        selectedPacket = packet;
    }

    // Move to the next page
    function nextPage(): void {
        if (currentPage < Math.ceil(allPackets.length / packetsPerPage)) {
            currentPage++;
            updateDisplayedPackets();
        }
    }

    // Move to the previous page
    function prevPage(): void {
        if (currentPage > 1) {
            currentPage--;
            updateDisplayedPackets();
        }
    }

    // Reactive statement to clear packets when the interface changes
    $: if (interfaceName) {
        allPackets = [];  // Clear packets when interface changes
        currentPage = 1;  // Reset the page
        displayedPackets = [];  // Clear displayed packets
        selectedPacket = null;  // Reset selected packet
    }

    $: totalPages = Math.ceil(allPackets.length / packetsPerPage);
</script>

<div class="container mx-auto p-4">
    <h2 class="text-2xl text-center font-bold mb-4">Packet Sniffer</h2>

    {#if error}
        <div class="bg-red-500 text-white p-2 rounded mb-4">
            {error}
        </div>
    {/if}

    <div class="mb-4 space-y-5 flex flex-col">
        <select
            bind:value={interfaceName}
            class="border text-black bg-gray-400 font-semibold rounded px-2 py-1 mr-2"
        >
            {#each availableInterfaces as iface}
                <option class="text-black" value={iface}>{iface}</option>
            {/each}
        </select>
        {#if !isCapturing}
            <button
                on:click={startCapture}
                class="bg-orange-700 mx-28 text-white font-semibold px-4 py-2 rounded hover:bg-orange-500"
            >
                Start Capture
            </button>
        {:else}
            <button
                on:click={stopCapture}
                class="bg-red-500 text-white px-4 py-2 rounded hover:bg-red-600"
            >
                Stop Capture
            </button>
        {/if}
    </div>

    <div class="flex">
        <div class="w-1/2 pr-2">
            <h3 class="text-xl font-semibold mb-2 text-center">Captured Packets</h3>
            <div class="h-96 overflow-y-auto border rounded">
                {#each displayedPackets as packet}
                    <div
                        on:click={() => selectPacket(packet)}
                        class="p-2 hover:bg-gray-100 cursor-pointer border-b"
                    >
                        <div class="text-sm">
                            {packet.src_mac} → {packet.dst_mac}
                        </div>
                        <div class="text-xs text-gray-600">
                            {packet.src_ip || 'Unknown'}:{packet.src_port || 'N/A'} → {packet.dst_ip || 'Unknown'}:{packet.dst_port || 'N/A'}
                        </div>
                    </div>
                {/each}
            </div>
            <div class="mt-2 flex justify-between items-center">
                <button
                    on:click={prevPage}
                    disabled={currentPage === 1}
                    class="bg-gray-300 text-gray-800 px-3 py-1 rounded disabled:opacity-50"
                >
                    Previous
                </button>
                <span>Page {currentPage} of {totalPages}</span>
                <button
                    on:click={nextPage}
                    disabled={currentPage === totalPages}
                    class="bg-gray-300 text-gray-800 px-3 py-1 rounded disabled:opacity-50"
                >
                    Next
                </button>
            </div>
        </div>

        <div class="w-1/2 pl-2">
            <h3 class="text-xl font-semibold mb-2 text-center">Packet Details</h3>
            {#if selectedPacket}
                <div class="border rounded p-4">
                    <pre class="text-xs overflow-x-auto whitespace-pre-wrap break-words">
                        {JSON.stringify(selectedPacket, null, 2)}
                    </pre>
                    {#if selectedPacket.dst_port === 80 && selectedPacket.payload}
                        <div class="mt-4">
                            <h4 class="text-lg font-semibold">HTTP Payload</h4>
                            <pre class="text-xs bg-gray-100 p-2 rounded mt-2">
                                {selectedPacket.payload}
                            </pre>
                        </div>
                    {/if}
                </div>
            {:else}
                <p class="text-gray-500">Select a packet to view details</p>
            {/if}
        </div>
    </div>
</div>