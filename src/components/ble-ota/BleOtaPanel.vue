<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import type {
  BleDfuDevice,
  BleOtaPackageInfo,
  BleOtaPhase,
  BleOtaProgressEvent,
} from "../../types/firmware";
import { resolveMessage } from "../../utils/backendI18n";
import BaseButton from "../ui/BaseButton.vue";
import BasePanel from "../ui/BasePanel.vue";
import BleDeviceList from "./BleDeviceList.vue";
import BleOtaProgress from "./BleOtaProgress.vue";

const { t } = useI18n();

const bluetoothAvailable = ref<boolean | null>(null);
const adapterHint = ref("");
const devices = ref<BleDfuDevice[]>([]);
const selectedPeripheralId = ref("");
const scanning = ref(false);

const zipFileName = ref("");
const zipBytes = ref<Uint8Array | null>(null);
const packageInfo = ref<BleOtaPackageInfo | null>(null);
const validationError = ref("");

const otaBusy = ref(false);
const phase = ref<BleOtaPhase | string>("idle");
const progress = ref(0);
const statusMessage = ref("");
const bytesTransferred = ref<number | undefined>(undefined);
const totalBytes = ref<number | undefined>(undefined);

let unlistenProgress: UnlistenFn | null = null;

const controlsLocked = computed(() => scanning.value || otaBusy.value);

const canStartOta = computed(
  () =>
    bluetoothAvailable.value === true &&
    !!selectedPeripheralId.value &&
    !!zipBytes.value &&
    !!packageInfo.value &&
    !controlsLocked.value,
);

async function refreshBluetooth(): Promise<void> {
  try {
    const res = await invoke<{
      available: boolean;
      adapters: { id: string; name: string }[];
    }>("check_bluetooth_available");
    bluetoothAvailable.value = res.available;
    adapterHint.value = res.adapters.map((a) => a.name).join(", ") || "";
  } catch {
    bluetoothAvailable.value = false;
    adapterHint.value = "";
  }
}

async function runScan(): Promise<void> {
  scanning.value = true;
  devices.value = [];
  selectedPeripheralId.value = "";
  try {
    devices.value = await invoke<BleDfuDevice[]>("scan_ble_dfu_devices");
  } catch (e) {
    statusMessage.value = typeof e === "string" ? e : t("ble_ota.scan_failed");
  } finally {
    scanning.value = false;
  }
}

function onSelectDevice(id: string): void {
  selectedPeripheralId.value = id;
}

function onZipChange(event: Event): void {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0] ?? null;
  packageInfo.value = null;
  validationError.value = "";
  zipBytes.value = null;
  zipFileName.value = "";
  if (!file) return;
  zipFileName.value = file.name;
  if (!file.name.toLowerCase().endsWith(".zip")) {
    validationError.value = t("ble_ota.invalid_zip");
    return;
  }
  void file.arrayBuffer().then((buf) => {
    zipBytes.value = new Uint8Array(buf);
    void validatePackage();
  });
}

async function validatePackage(): Promise<void> {
  if (!zipBytes.value?.length) return;
  try {
    const info = await invoke<BleOtaPackageInfo>("validate_ota_package", {
      zipBytes: Array.from(zipBytes.value),
    });
    packageInfo.value = info;
    validationError.value = "";
  } catch (e) {
    packageInfo.value = null;
    validationError.value = resolveMessage(t, typeof e === "string" ? e : String(e));
  }
}

async function startOta(): Promise<void> {
  if (!zipBytes.value || !selectedPeripheralId.value) return;
  otaBusy.value = true;
  phase.value = "connecting";
  progress.value = 0;
  statusMessage.value = t("ble_ota.starting");
  bytesTransferred.value = undefined;
  totalBytes.value = undefined;
  try {
    await invoke("start_ble_ota", {
      peripheralId: selectedPeripheralId.value,
      zipBytes: Array.from(zipBytes.value),
    });
  } catch (e) {
    statusMessage.value = resolveMessage(t, typeof e === "string" ? e : String(e));
    phase.value = "error";
  } finally {
    otaBusy.value = false;
  }
}

async function cancelOta(): Promise<void> {
  try {
    await invoke("cancel_ble_ota");
  } catch {
    /* ignore */
  }
}

function formatSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

onMounted(async () => {
  await refreshBluetooth();
  unlistenProgress = await listen<BleOtaProgressEvent>("ble-ota-progress", (ev) => {
    const p = ev.payload;
    phase.value = p.phase as BleOtaPhase;
    progress.value = p.progress;
    statusMessage.value = resolveMessage(t, p.message);
    bytesTransferred.value = p.bytesTransferred;
    totalBytes.value = p.totalBytes;
  });
});

onUnmounted(() => {
  void unlistenProgress?.();
});
</script>

<template>
  <div class="ble-ota-panel">
    <BasePanel :title="t('ble_ota.panel_title')">
      <p class="intro">{{ t("ble_ota.intro") }}</p>

      <div class="bt-block">
        <p class="bt-status">
          <template v-if="bluetoothAvailable === null">{{ t("ble_ota.bt_checking") }}</template>
          <template v-else-if="bluetoothAvailable">{{ t("ble_ota.bt_ok") }} {{ adapterHint }}</template>
          <template v-else>{{ t("ble_ota.bt_missing") }}</template>
        </p>
        <BaseButton variant="outline" :disabled="controlsLocked" @click="refreshBluetooth">
          {{ t("ble_ota.refresh_bt") }}
        </BaseButton>
      </div>

      <div class="scan-block">
        <BaseButton variant="outline" :disabled="controlsLocked || bluetoothAvailable !== true" @click="runScan">
          {{ scanning ? t("ble_ota.scanning") : t("ble_ota.scan") }}
        </BaseButton>
        <span class="hint">{{ t("ble_ota.device_names_hint") }}</span>
      </div>

      <BleDeviceList
        :devices="devices"
        :selected-id="selectedPeripheralId"
        :disabled="controlsLocked"
        @select="onSelectDevice"
      />

      <div class="zip-block">
        <label class="zip-label" for="bleOtaZip">{{ t("ble_ota.zip_label") }}</label>
        <input
          id="bleOtaZip"
          class="file-input"
          type="file"
          accept=".zip"
          :disabled="controlsLocked"
          @change="onZipChange"
        />
        <div v-if="zipFileName" class="file-card">
          <div class="file-meta">
            <span>{{ t("flashing.file_name_label") }}</span>
            <strong class="mono file-name">{{ zipFileName }}</strong>
          </div>
          <template v-if="packageInfo">
            <p class="pkg-ok">{{ t("ble_ota.package_ok") }}</p>
            <div class="file-meta">
              <span>{{ t("ble_ota.bin_file") }}</span>
              <strong class="mono">{{ packageInfo.binFileName }} ({{ formatSize(packageInfo.binSize) }})</strong>
            </div>
            <div class="file-meta">
              <span>{{ t("ble_ota.dat_file") }}</span>
              <strong class="mono">{{ packageInfo.datFileName }} ({{ formatSize(packageInfo.datSize) }})</strong>
            </div>
          </template>
        </div>
        <p v-if="validationError" class="err">{{ validationError }}</p>
      </div>

      <div class="actions">
        <BaseButton :disabled="!canStartOta" @click="startOta">
          {{ t("ble_ota.start_upgrade") }}
        </BaseButton>
        <BaseButton variant="outline" :disabled="!otaBusy" @click="cancelOta">
          {{ t("ble_ota.cancel") }}
        </BaseButton>
      </div>
    </BasePanel>

    <BasePanel :title="t('ble_ota.progress_title')">
      <BleOtaProgress
        :phase="phase"
        :progress="progress"
        :message="statusMessage || t('ble_ota.idle_hint')"
        :bytes-transferred="bytesTransferred"
        :total-bytes="totalBytes"
      />
    </BasePanel>
  </div>
</template>

<style scoped>
.ble-ota-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}
.intro {
  margin: 0 0 var(--spacing-sm);
  font-size: 12px;
  line-height: 1.45;
  color: var(--color-text-light);
}
.bt-block {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-sm);
}
.bt-status {
  margin: 0;
  flex: 1;
  font-size: 12px;
  min-width: 0;
}
.scan-block {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-md);
}
.hint {
  font-size: 11px;
  color: var(--color-text-light);
}
.zip-block {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  margin-top: var(--spacing-md);
}
.zip-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
}
.pkg-ok {
  margin: 0;
  font-size: 12px;
  color: var(--color-success);
  font-weight: 600;
}
.err {
  margin: 0;
  font-size: 12px;
  color: var(--color-error);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-md);
}
</style>
