# Compatibilidade de Hardware — DeckStudio

O **DeckStudio** é uma alternativa **100% gratuita e open-source (Licença MIT)** desenvolvida para substituir softwares pesados baseados em QtWebEngine/Electron, falando diretamente com a interface **HID vendor-defined (`usage_page >= 0xFF00`)** via pacotes seguros de 512 bytes (JPEG RGB ≤ 96 px, ≤ 10 KiB, sem EXIF/ICC).

> **Aviso de transparência:** diferenciamos explicitamente o hardware **testado fisicamente em bancada** dos **modelos candidatos** que compartilham a mesma plataforma OEM Mirabox (`mirajazz` / `opendeck-akp153`). Modelos ainda não testados em bancada exigem validação do mapa de botões e dos descritores HID (`512 B` vs `1024 B`, protocolo v1 vs v2).

---

## 1. Matriz de Compatibilidade

| Família / Modelo | VID : PID | Layout Físico | Protocolo HID Esperado | Status Atual |
| :--- | :--- | :--- | :--- | :--- |
| **AJAZZ AKP153E** | `0x0300 : 0x1010` | 15 teclas LCD (5×3) + faixa lateral 3×1 | Report 512 B + `0x00` report ID, JPEG 96×96 rotacionado 90° CCW, entrada v1 press-only (`0x00`) | **Testado & Verificado (100%)** |
| **AJAZZ AKP153** | `0x5548 : 0x6674` | 15 teclas LCD (5×3) + faixa lateral | Plataforma Mirabox V1 (muitas revisões usam payload 512 B / v1) | Candidato OEM Mirabox |
| **AJAZZ AKP153E / AKP153R (outras rev.)** | `0x0300 : 0x1020`, `0x3010`, `0x3011` | 15 teclas LCD (5×3) + faixa lateral | Família v2 (`0x01` press / `0x00` release já suportado em `parse_input_report`, verificar tamanho de pacote `512 B` vs `1024 B`) | Candidato OEM Mirabox |
| **Mirabox StreamDock HSV293S** | `0x5548 : 0x6670` | 15 teclas LCD (5×3) + strip | Hardware OEM irmão do AKP153 (firmware v1/v2) | Candidato OEM Mirabox |
| **Mirabox StreamDock HSV293SV3 / 293V3** | `0x6603 : 0x1014`, `0x6603 : 0x1005` | 15 teclas LCD (5×3) + strip | Revisão 6603 (`parse_input_report` já reconhece v2 press/release) | Candidato OEM Mirabox |
| **Rebadges OEM Mirabox (15 teclas)** *(Mars Gaming MSD-One, Soomfon CN002, Womier, RiseMode, Monstargear)* | Variados (`0x0300`, `0x5548`, `0x6603`) | 15 teclas LCD (5×3) + variantes | Mesmo pipeline `CRT..` / JPEG por posição com possíveis diferenças de `OutputReportByteLength` e rotação | Candidato (em análise) |
| **AJAZZ AKP03 / VSDinside StreamDock N3 · N4** | `0x0300 : 0x1001/1002`, `0x5548 : 0x1001/1023`, `0x6603 : 0x1002/1003/1007` | Teclas LCD + **Encoders Rotativos (Knobs)** | Requer suporte dedicado a eventos de rotação dos knobs e tela de barra contínua | **Roadmap Futuro** |
| **Elgato Stream Deck (MK.2, XL, Mini, Plus)** / **Loupedeck** | `0x0FD9` / outros | Diversos | Protocolos proprietários completamente distintos da família `CRT..` Mirabox | **Incompatível** (não suportado) |

---

## 2. O que muda entre revisões da família Mirabox / AJAZZ

1. **`OutputReportByteLength` (512 B vs 1024 B):**
   - No **AJAZZ AKP153E (`0300:1010`)**, o descritor HID do Windows exige `1 + 512 = 513 bytes`. Enviar pacotes de 1024 bytes falha silenciosamente ou gera erro na API HID do Windows.
   - Algumas revisões `0x5548` / `0x6603` esperam `1024 bytes` (`REPORT_PAYLOAD_SIZE = 1024`). Tornar a tabela de dispositivos configurável em [hid_proto.rs](../deckstudio/crates/deck-engine/src/hid_proto.rs) permitirá habilitar todos os VID/PIDs candidatos acima com segurança.
2. **Protocolo de Entrada (`v1` vs `v2`):**
   - O `parse_input_report()` em [hid_proto.rs](../deckstudio/crates/deck-engine/src/hid_proto.rs) já suporta os dois modos: `v1` (pacote único `0x00` por clique) e `v2` (`0x01` ao pressionar, `0x00` ao soltar).
3. **Orientação e Resolução do Tile:**
   - O AKP153E usa `96×96 px` rotacionado `90°` anti-horário (`rotate270()` no crate `image`), RGB puro (sem alfa), sem metadados EXIF/ICC e limite rígido de `10 KiB`.

---

## 3. Como validar um modelo novo com segurança

Se quiser testar outro controlador OEM Mirabox antes de abrir suporte oficial:

1. **Feche o software oficial** (`Stream Dock AJAZZ.exe`, `StreamDock.exe`, `VSD Craft.exe`) para liberar a interface HID exclusiva.
2. Inspecione no Windows o VID, PID e `OutputReportByteLength` usando `hidapi`:
   ```powershell
   C:\Users\wagne\workspace\StreamDeck\.venv\Scripts\python.exe -c "import hid; [print(hex(d['vendor_id']), hex(d['product_id']), hex(d['usage_page']), d['product_string']) for d in hid.enumerate() if d['usage_page'] >= 0xFF00]"
   ```
3. Nunca envie comandos não documentados (logo de boot, firmware, sleep/wakeup ou reset). Use apenas a sequência segura (`INIT`, `BACKLIGHT`, `CLEAR_ALL`, `IMG_ANNOUNCE` + chunks + `FLUSH_BUFFER`).
4. Adicione a nova entrada na matriz deste arquivo, na aba **Sobre** em [App.tsx](../deckstudio/apps/studio/src/App.tsx) e nos diagramas Archify (`architecture` / `compatibility`).
