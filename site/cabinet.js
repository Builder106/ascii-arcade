/*
 * The cabinet: an arcade machine modelled in Blender
 * (assets/cabinet/arcade-cabinet.blend, compressed by
 * scripts/optimize-cabinet.sh into models/cabinet.glb). Its screen is the
 * live grid, and the page paints the marquee, side art, panel art, price
 * windows and coin label in the theme colour. Each station frames a part of
 * the machine; only playing DOOM pushes the camera through the glass.
 *
 * It loads after the page is up and is never needed: without WebGL 2, under
 * reduced motion, or if this module or the model fails, the flat page is the
 * whole site.
 */

import * as THREE from "./vendor/three.min.js";
import { GLTFLoader, MeshoptDecoder, RoomEnvironment } from "./vendor/three.min.js";
import { FONT as BLOCKS } from "./boot.js";

const MODEL = new URL("./models/cabinet.glb", import.meta.url).href;
const FONT = '"IBM Plex Mono", ui-monospace, Menlo, monospace';
// The page's display face, for the cabinet's small labels.
const SIGN = '"Pixelify Sans", "IBM Plex Mono", monospace';
const RAMP = ".,-~:;=!*#$@";
const SCREEN_W = 0.5; // the tube's size in the model, in metres
const SCREEN_H = 0.375;
const FOV = 30;

const ease = (k) => (k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2);

/**
 * Draws a sign in the boot logo's 5 by 5 block letters, centred on (cx, cy)
 * and as wide as fits in maxWidth. Pixels are a little taller than wide, the
 * way marquee lettering is drawn, with a hairline between them.
 */
function blockText(ctx, text, cx, cy, maxWidth) {
  const units = text.length * 6 - 1;
  const uw = Math.floor(maxWidth / units);
  const uh = Math.round(uw * 1.3);
  const left = Math.round(cx - (units * uw) / 2);
  const top = Math.round(cy - (5 * uh) / 2);
  [...text].forEach((ch, i) => {
    const glyph = BLOCKS[ch];
    for (let y = 0; y < 5; y++) {
      for (let x = 0; x < 5; x++) {
        if (glyph[y][x] === "1") ctx.fillRect(left + (i * 6 + x) * uw, top + y * uh, uw - 1, uh - 1);
      }
    }
  });
}

function canvas2d(w, h) {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  return [c, c.getContext("2d")];
}

// glTF UVs run top-down, so textures painted onto the model don't flip.
function texture(c, flipY = false) {
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  t.flipY = flipY;
  return t;
}

export async function mountCabinet(screenCanvas, colors) {
  const gltf = await new GLTFLoader().setMeshoptDecoder(MeshoptDecoder).loadAsync(MODEL);
  const cab = gltf.scene;
  const node = (name) => {
    const n = cab.getObjectByName(name);
    if (!n) throw new Error(`cabinet model has no ${name}`);
    return n;
  };

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio || 1, innerWidth < 700 ? 1.25 : 1.5));
  renderer.toneMapping = THREE.NeutralToneMapping;
  const el = renderer.domElement;
  el.id = "cabinet";
  el.setAttribute("aria-hidden", "true");
  document.getElementById("wall").after(el);

  const scene = new THREE.Scene();
  scene.fog = new THREE.Fog(0x000000, 5, 11);
  const camera = new THREE.PerspectiveCamera(FOV, 1, 0.01, 40);
  scene.add(cab);

  // Studio reflections, given only to the metal, the gloss and the glass:
  // sampling them on every surface costs far more than it shows.
  const pmrem = new THREE.PMREMGenerator(renderer);
  const studio = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  pmrem.dispose();

  // Every mesh gets its own copy of its material, so a hovered part can glow
  // without lighting up the other parts that share its finish. Themed and
  // painted materials are then found again by name.
  const byName = new Map();
  cab.traverse((o) => {
    if (!o.isMesh) return;
    o.material = o.material.clone();
    const m = o.material;
    if (!byName.has(m.name)) byName.set(m.name, []);
    byName.get(m.name).push(m);
    if (m.isMeshStandardMaterial && (m.metalness > 0.5 || m.roughness < 0.3)) {
      m.envMap = studio;
      m.envMapIntensity = 0.45;
    }
    if (m.name === "MAT_Glass") {
      m.transparent = true;
      m.depthWrite = false;
    }
  });
  const swap = (name, make) => {
    cab.traverse((o) => {
      if (o.isMesh && o.material.name === name) {
        o.material = make();
        o.material.name = name;
      }
    });
  };
  const painted = (tex) => () => new THREE.MeshBasicMaterial({ map: tex, toneMapped: false });

  // The screen: the live grid, with faint scanlines drawn over the same tube.
  const screen = node("SCREEN");
  const screenTex = texture(screenCanvas);
  screenTex.minFilter = THREE.LinearFilter;
  screenTex.generateMipmaps = false;
  swap("MAT_Screen", painted(screenTex));
  const [lineCanvas, lineCtx] = canvas2d(4, 4);
  lineCtx.fillStyle = "rgba(0,0,0,0.38)";
  lineCtx.fillRect(0, 2, 4, 2);
  const lineTex = texture(lineCanvas);
  lineTex.wrapS = lineTex.wrapT = THREE.RepeatWrapping;
  lineTex.repeat.set(1, 140);
  const lines = new THREE.Mesh(
    screen.geometry,
    new THREE.MeshBasicMaterial({ map: lineTex, transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -1 }),
  );
  screen.add(lines);

  const [sideCanvas, sideCtx] = canvas2d(512, 1100);
  const sideTex = texture(sideCanvas);
  swap("MAT_SideArt", () => new THREE.MeshStandardMaterial({ map: sideTex, roughness: 0.6 }));
  const [marqueeCanvas, marqueeCtx] = canvas2d(1024, 340);
  const marqueeTex = texture(marqueeCanvas);
  swap("MAT_Marquee", painted(marqueeTex));
  const [panelCanvas, panelCtx] = canvas2d(1024, 384);
  const panelTex = texture(panelCanvas);
  swap("MAT_PanelArt", () => new THREE.MeshStandardMaterial({ map: panelTex, roughness: 0.4 }));
  const [priceCanvas, priceCtx] = canvas2d(256, 80);
  const priceTex = texture(priceCanvas);
  swap("MAT_Price", painted(priceTex));
  const [coinCanvas, coinCtx] = canvas2d(512, 96);
  const coinTex = texture(coinCanvas);
  swap("MAT_CoinLabel", painted(coinTex));
  swap("MAT_Slot", () => new THREE.MeshBasicMaterial({ color: 0xff7a1a, toneMapped: false }));

  // Lamps: the lit parts, which start dark and flicker on with the first
  // reveal, the way a cabinet's fluorescent tubes catch when it's switched on.
  const lamps = [];
  cab.traverse((o) => {
    if (o.isMesh && ["MAT_Marquee", "MAT_Price", "MAT_CoinLabel", "MAT_Slot"].includes(o.material.name)) {
      o.material.userData.lamp = o.material.color.clone();
      lamps.push(o.material);
    }
  });
  const FLICKER = [[0, 0], [0.06, 0.8], [0.1, 0.05], [0.18, 0.6], [0.22, 0.1], [0.32, 1], [0.36, 0.35], [0.46, 1]];
  let power = 0;
  let powerT = -1;
  let powerLength = 1;
  function applyPower() {
    for (const m of lamps) m.color.copy(m.userData.lamp).multiplyScalar(power);
    for (const name of ["MAT_Trim", "MAT_Button"]) {
      for (const m of byName.get(name) ?? []) m.emissiveIntensity = (m.userData.lampI ?? 0) * power;
    }
    glow.intensity = 2.2 * power;
  }

  // Controls the page moves: the stick tilts about the panel's up axis, the
  // buttons press into the panel, and a coin drops into the right-hand slot.
  // The model's nodes keep Blender's local axes converted to Y-up, so the
  // panel's normal is local +Y and its up axis is local -Z.
  const stick = node("STICK");
  const stickRest = stick.quaternion.clone();
  const stickBall = node("STICK_BALL");
  const PANEL_UP = new THREE.Vector3(0, 0, -1);
  const STICK_STOP = 0.45; // radians
  const buttons = [node("BUTTON_0"), node("BUTTON_1")];
  const buttonRest = buttons.map((b) => b.position.y);
  const coin = new THREE.Mesh(
    new THREE.CylinderGeometry(0.012, 0.012, 0.003, 24),
    new THREE.MeshStandardMaterial({ color: 0xd4af37, metalness: 0.95, roughness: 0.25, envMap: studio }),
  );
  coin.rotation.z = Math.PI / 2;
  coin.visible = false;
  node("COIN_SLOT_R").add(coin);

  // Room: a floor that fades into fog, a soft shadow, and three lights.
  const floorMat = new THREE.MeshStandardMaterial({ roughness: 0.95 });
  const floor = new THREE.Mesh(new THREE.PlaneGeometry(40, 40), floorMat);
  floor.rotation.x = -Math.PI / 2;
  scene.add(floor);
  const [shadowCanvas, shadowCtx] = canvas2d(128, 128);
  const grad = shadowCtx.createRadialGradient(64, 64, 4, 64, 64, 64);
  grad.addColorStop(0, "rgba(0,0,0,0.75)");
  grad.addColorStop(1, "rgba(0,0,0,0)");
  shadowCtx.fillStyle = grad;
  shadowCtx.fillRect(0, 0, 128, 128);
  const shadow = new THREE.Mesh(
    new THREE.PlaneGeometry(1.6, 1.6),
    new THREE.MeshBasicMaterial({ map: texture(shadowCanvas, true), transparent: true, depthWrite: false }),
  );
  shadow.rotation.x = -Math.PI / 2;
  shadow.position.y = 0.002;
  scene.add(shadow);

  scene.add(new THREE.HemisphereLight(0xffffff, 0x000000, 0.4));
  const key = new THREE.SpotLight(0xffffff, 70, 14, 0.5, 0.6);
  key.position.set(1.8, 3.6, 2.6);
  key.target = cab;
  scene.add(key);
  const glow = new THREE.PointLight(0xffffff, 2.2, 2.2);
  glow.position.set(0, 1.25, 0.6);
  scene.add(glow);

  function paintSide(ink) {
    const { width: w, height: h } = sideCanvas;
    sideCtx.fillStyle = "#0b0b0b";
    sideCtx.fillRect(0, 0, w, h);
    sideCtx.font = `600 30px ${FONT}`;
    sideCtx.fillStyle = ink;
    for (let y = 0; y < h; y += 34) {
      for (let x = 0; x < w; x += 19) {
        const k = Math.min(1, Math.max(0, 1.15 - (x / w) * 0.55 - (y / h) * 0.9 + Math.sin(y * 0.02) * 0.05));
        sideCtx.fillText(RAMP[Math.floor(k * (RAMP.length - 1))], x, y + 28);
      }
    }
    sideTex.needsUpdate = true;
  }

  function paintMarquee(ink) {
    const { width: w, height: h } = marqueeCanvas;
    marqueeCtx.fillStyle = "#050505";
    marqueeCtx.fillRect(0, 0, w, h);
    marqueeCtx.font = `600 22px ${FONT}`;
    marqueeCtx.fillStyle = ink;
    marqueeCtx.globalAlpha = 0.14;
    for (let y = 0; y < h; y += 24) {
      for (let x = 0; x < w; x += 14) {
        const k = Math.abs(Math.sin(x * 0.012 + y * 0.03)) * (1 - Math.abs(y / h - 0.5));
        marqueeCtx.fillText(RAMP[Math.floor(k * (RAMP.length - 1))], x, y + 18);
      }
    }
    marqueeCtx.globalAlpha = 1;
    marqueeCtx.textAlign = "center";
    marqueeCtx.textBaseline = "middle";
    marqueeCtx.shadowColor = ink;
    marqueeCtx.shadowBlur = 28;
    marqueeCtx.fillStyle = ink;
    blockText(marqueeCtx, "ASCII ARCADE", w / 2, h / 2 - 12, w - 60);
    marqueeCtx.shadowBlur = 0;
    marqueeCtx.font = `500 36px ${SIGN}`;
    marqueeCtx.fillText("live wallpaper for macOS", w / 2, h - 46);
    marqueeTex.needsUpdate = true;
  }

  // Panel art, laid out to the model: the stick sits 0.15 m left of centre,
  // the buttons 0.06 and 0.15 m right, the 1P button 0.25 m right.
  function paintPanel(ink) {
    const { width: w, height: h } = panelCanvas;
    const px = (x) => ((x + 0.32) / 0.64) * w;
    const py = (y) => h / 2 - (y / 0.24) * h;
    panelCtx.fillStyle = "#0c0c0c";
    panelCtx.fillRect(0, 0, w, h);
    panelCtx.strokeStyle = ink;
    panelCtx.globalAlpha = 0.45;
    panelCtx.lineWidth = 4;
    panelCtx.strokeRect(18, 18, w - 36, h - 36);
    panelCtx.globalAlpha = 0.8;
    panelCtx.fillStyle = ink;
    panelCtx.textAlign = "center";
    panelCtx.textBaseline = "middle";
    panelCtx.font = `600 44px ${FONT}`;
    for (const [ch, dx, dy] of [["↑", 0, -105], ["↓", 0, 105], ["←", -110, 0], ["→", 110, 0]]) panelCtx.fillText(ch, px(-0.15) + dx, py(0) + dy);
    panelCtx.font = `600 32px ${SIGN}`;
    panelCtx.fillText("SCENE", px(0.06), py(0.015) + 82);
    panelCtx.fillText("START", px(0.15), py(0.015) + 82);
    panelCtx.fillText("1P", px(0.25), py(0.085) + 48);
    panelCtx.globalAlpha = 1;
    panelTex.needsUpdate = true;
  }

  function paintPrice(ink) {
    const { width: w, height: h } = priceCanvas;
    priceCtx.fillStyle = "#060606";
    priceCtx.fillRect(0, 0, w, h);
    priceCtx.fillStyle = ink;
    priceCtx.textAlign = "center";
    priceCtx.textBaseline = "middle";
    priceCtx.font = `600 42px ${SIGN}`;
    priceCtx.fillText("FREE PLAY", w / 2, h / 2 + 2, w - 16);
    priceTex.needsUpdate = true;
  }

  function paintCoin(ink) {
    const { width: w, height: h } = coinCanvas;
    coinCtx.fillStyle = "#0a0a0a";
    coinCtx.fillRect(0, 0, w, h);
    coinCtx.textAlign = "center";
    coinCtx.textBaseline = "middle";
    coinCtx.shadowColor = ink;
    coinCtx.shadowBlur = 16;
    coinCtx.fillStyle = ink;
    blockText(coinCtx, "INSERT COIN", w / 2, h / 2, w - 24);
    coinTex.needsUpdate = true;
  }

  let inkColor = new THREE.Color(colors.ink);
  function setTheme({ ground, ink }) {
    const g = new THREE.Color(ground);
    inkColor = new THREE.Color(ink);
    scene.background = g;
    scene.fog.color = g;
    floorMat.color = g.clone().lerp(new THREE.Color(0x808080), 0.08);
    for (const [name, intensity] of [["MAT_Trim", 0.22], ["MAT_Button", 0.35]]) {
      for (const m of byName.get(name) ?? []) {
        m.color.copy(inkColor);
        m.emissive.copy(inkColor);
        m.userData.lampI = intensity;
        m.emissiveIntensity = intensity * power;
        m.userData.base = undefined;
      }
    }
    glow.color = inkColor;
    glow.intensity = 2.2 * power;
    paintSide(ink);
    paintMarquee(ink);
    paintCoin(ink);
    paintPanel(ink);
    paintPrice(ink);
  }

  // Camera poses, one per station: what the camera looks at, the direction
  // it looks from, and how much has to fit (height on wide screens, width on
  // tall ones). The subject sits right of the copy on wide screens and above
  // it on tall ones. "glass" is square to the screen, close enough that the
  // glass covers the window.
  const POSES = {
    title: { at: [0, 0.92, 0], from: [0.42, 0.14, 1], h: 2.6, w: 1.55 },
    demo: { at: [0, 1.36, 0.1], from: [0.12, 0.32, 0.94], h: 1.15, w: 0.95 },
    scenes: { at: [0, 1.12, 0.22], from: [0.3, 0.85, 1], h: 1.25, w: 1.05 },
    rules: { at: [0.2, 0.95, 0], from: [1, 0.12, -0.3], h: 2.4, w: 1.7 },
    install: { at: [0, 0.55, 0.31], from: [-0.3, 0.12, 1], h: 1.05, w: 0.85 },
  };
  const UP = THREE.Object3D.DEFAULT_UP;
  const m4 = new THREE.Matrix4();
  const screenCentre = node("SCREEN_CENTER");
  const screenUp = node("SCREEN_UP");
  const screenOut = node("SCREEN_OUT");

  function glassPose() {
    const centre = screenCentre.getWorldPosition(new THREE.Vector3());
    const up = screenUp.getWorldPosition(new THREE.Vector3()).sub(centre).normalize();
    const normal = screenOut.getWorldPosition(new THREE.Vector3()).sub(centre).normalize();
    const half = Math.tan(THREE.MathUtils.degToRad(FOV) / 2);
    const d = Math.min(SCREEN_H / (2 * half), SCREEN_W / (2 * half * camera.aspect)) * 0.97;
    const pos = centre.clone().addScaledVector(normal, d);
    m4.lookAt(pos, centre, up);
    return { pos, quat: new THREE.Quaternion().setFromRotationMatrix(m4) };
  }

  function pose(name, yaw) {
    if (name === "glass") return glassPose();
    const P = POSES[name];
    const a = camera.aspect;
    const half = Math.tan(THREE.MathUtils.degToRad(FOV) / 2);
    const subject = new THREE.Vector3(...P.at);
    const dist = a >= 1 ? P.h / (2 * half) : P.w / (2 * half * a);
    const dir = new THREE.Vector3(...P.from).normalize().applyAxisAngle(UP, yaw);
    const pos = subject.clone().addScaledVector(dir, dist);
    const quat = new THREE.Quaternion().setFromRotationMatrix(m4.lookAt(pos, subject, UP));
    const visH = 2 * dist * half;
    const shift = a >= 1
      ? new THREE.Vector3(1, 0, 0).applyQuaternion(quat).multiplyScalar(-0.23 * visH * a)
      : new THREE.Vector3(0, 1, 0).applyQuaternion(quat).multiplyScalar(-0.2 * visH);
    m4.lookAt(pos, subject.add(shift), UP);
    return { pos, quat: quat.setFromRotationMatrix(m4) };
  }

  // Moves orbit the cabinet: direction and distance from its middle are
  // interpolated separately, so the camera swings round the machine instead
  // of cutting through it on the way from one side to another.
  const CENTRE = new THREE.Vector3(0, 0.92, 0);
  function orbit(out, a, b, e) {
    const va = a.clone().sub(CENTRE);
    const vb = b.clone().sub(CENTRE);
    const la = va.length();
    const lb = vb.length();
    const turn = new THREE.Quaternion().setFromUnitVectors(va.normalize(), vb.normalize());
    const part = new THREE.Quaternion().slerp(turn, e);
    return out.copy(va).applyQuaternion(part).multiplyScalar(la + (lb - la) * e).add(CENTRE);
  }

  function resize() {
    renderer.setSize(innerWidth, innerHeight, false);
    camera.aspect = innerWidth / innerHeight;
    camera.updateProjectionMatrix();
  }

  // The camera starts at the glass, so the first move pulls out of the screen.
  let from = null;
  let fromName = "glass";
  let toName = "glass";
  let t = 1;
  let length = 1;
  let done = null;
  let time = 0;
  let sinceRender = 1;
  const pointer = { x: 0, tx: 0 };
  // The camera drifts with the mouse, except while the stick is under it,
  // so the stick stays where the pointer found it.
  addEventListener("pointermove", (e) => {
    if (e.pointerType === "mouse" && stickHeld === null) pointer.tx = e.clientX / innerWidth - 0.5;
  });

  /** Renders a frame and returns how far through the glass the camera is, 0 to 1. */
  function update(dt) {
    time += dt;
    const controlsMoving = animateControls(dt);
    if (powerT >= 0) {
      powerT = Math.min(1, powerT + dt / powerLength);
      let v = 0;
      for (const [at, value] of FLICKER) if (powerT >= at) v = value;
      power = powerT >= 1 ? 1 : v;
      applyPower();
      if (powerT >= 1) powerT = -1;
    }
    t = Math.min(1, t + dt / length);
    const e = ease(t);
    const glass = toName === "glass" ? e : fromName === "glass" ? 1 - e : 0;
    if (t >= 1 && done) {
      const cb = done;
      done = null;
      cb();
    }
    if (glass >= 1 && t >= 1) return 1; // the flat page is showing the screen
    // At rest the screen only changes 30 times a second, so the cabinet
    // renders at that rate too; while anything moves it renders every frame.
    sinceRender += dt;
    const moving = t < 1 || controlsMoving || powerT >= 0 || Math.abs(pointer.tx - pointer.x) > 0.002;
    if (!moving && sinceRender < 1 / 30) return glass;
    sinceRender = 0;
    pointer.x += (pointer.tx - pointer.x) * 0.06;
    const yaw = (pointer.x * 0.22 + Math.sin(time * 0.3) * 0.04) * (1 - glass);
    const target = pose(toName, yaw);
    if (!from) from = target;
    orbit(camera.position, from.pos, target.pos, e);
    camera.quaternion.slerpQuaternions(from.quat, target.quat, e);
    lines.material.opacity = 1 - glass;
    renderer.render(scene, camera);
    return glass;
  }

  /** Moves the camera to a station's pose, or "glass", then calls `cb`. */
  function frame(name, seconds, cb) {
    from = { pos: camera.position.clone(), quat: camera.quaternion.clone() };
    if (t >= 1 && fromName === "glass" && toName === "glass") from = glassPose();
    fromName = toName;
    toName = name;
    t = 0;
    length = Math.max(seconds, 0.01);
    done = cb ?? null;
  }

  // Controls. The stick tilts and springs back, buttons press, a coin drops
  // into the slot, and the part under the pointer answers for its station.
  const raycaster = new THREE.Raycaster();
  const ndc = new THREE.Vector2();
  let stickHold = 0;
  let stickTarget = 0;
  let stickAngle = 0;
  let stickHeld = null; // the angle the pointer holds the stick at, if any
  const pressed = [0, 0];
  let coinT = -1;
  let coinDone = null;
  let hovered = null;

  // Each station's part of the model is a PART_<station> group; the coin
  // slots and the stick are their own targets. Anything else (the body)
  // answers nothing.
  function tagOf(o) {
    for (let n = o; n; n = n.parent) {
      if (n.name === "STICK") return "stick";
      if (n.name.startsWith("COIN_SLOT")) return "coin";
      if (n.name.startsWith("PART_")) return n.name.slice(5);
    }
    return null;
  }

  function pick(x, y) {
    ndc.set((x / innerWidth) * 2 - 1, -(y / innerHeight) * 2 + 1);
    raycaster.setFromCamera(ndc, camera);
    const hit = raycaster.intersectObject(cab, true).find((h) => h.object !== lines);
    return hit ? tagOf(hit.object) : null;
  }

  // A hovered part lifts toward the theme colour.
  function hover(tag) {
    if (tag === hovered) return;
    hovered = tag;
    cab.traverse((o) => {
      if (!o.isMesh || o === lines) return;
      const on = tag !== null && tagOf(o) === tag;
      const m = o.material;
      m.userData.base ??= m.isMeshBasicMaterial
        ? (m.userData.lamp ?? m.color).clone()
        : { e: m.emissive.clone(), i: m.userData.lampI ?? m.emissiveIntensity };
      if (m.isMeshBasicMaterial) m.color.copy(m.userData.base).multiplyScalar(on ? 1.6 : 1);
      else if (on) {
        m.emissive.copy(inkColor);
        m.emissiveIntensity = Math.max(m.userData.base.i, 0.06);
      } else {
        m.emissive.copy(m.userData.base.e);
        m.emissiveIntensity = m.userData.base.i;
      }
    });
    sinceRender = 1;
  }

  // Where the ball lands on screen, in pixels, with the stick at angle a.
  const tilted = new THREE.Quaternion();
  const ballAt = new THREE.Vector3();
  function ballX(a) {
    stick.quaternion.copy(stickRest).multiply(tilted.setFromAxisAngle(PANEL_UP, a));
    stick.updateMatrixWorld(true);
    stickBall.getWorldPosition(ballAt).project(camera);
    return ((ballAt.x + 1) / 2) * innerWidth;
  }

  // Leans the stick a little toward the pointer's x, for a hover. Works out
  // the stick's length on screen from two trial angles, so the lean points
  // the right way from any camera pose.
  function leanStick(x) {
    const rest = ballX(0);
    const length = (ballX(0.2) - rest) / Math.sin(0.2);
    stick.quaternion.copy(stickRest).multiply(tilted.setFromAxisAngle(PANEL_UP, stickAngle));
    holdStick(Math.asin(Math.min(1, Math.max(-1, (x - rest) / length))) * 0.3);
  }

  // Holds the stick at an angle, clamped to its stop. Positive leans it the
  // way → steps.
  function holdStick(a) {
    stickHeld = Math.min(STICK_STOP, Math.max(-STICK_STOP, a));
    sinceRender = 1;
  }

  // Returns whether anything is still moving, so the cabinet keeps rendering.
  function animateControls(dt) {
    stickHold = Math.max(0, stickHold - dt);
    const target = stickHeld ?? (stickHold > 0 ? stickTarget : 0);
    stickAngle += (target - stickAngle) * Math.min(1, dt * 18);
    stick.quaternion.copy(stickRest).multiply(new THREE.Quaternion().setFromAxisAngle(PANEL_UP, stickAngle));
    buttons.forEach((b, i) => {
      pressed[i] = Math.max(0, pressed[i] - dt);
      b.position.y = buttonRest[i] - (pressed[i] > 0 ? 0.006 : 0);
    });
    if (coinT >= 0) {
      coinT += dt / 0.5;
      coin.visible = coinT < 1;
      coin.position.set(0, 0.06 * (1 - ease(Math.min(coinT, 1))) + 0.002, 0);
      if (coinT >= 1) {
        coinT = -1;
        const cb = coinDone;
        coinDone = null;
        cb?.();
      }
    }
    return coinT >= 0 || stickHold > 0 || Math.abs(stickAngle - target) > 0.002 || Math.abs(stickAngle) > 0.002 || pressed.some((p) => p > 0);
  }

  // Canvas text only uses a web font once it has loaded.
  await document.fonts.load(`600 32px ${SIGN}`).catch(() => {});
  setTheme(colors);
  resize();
  addEventListener("resize", resize);
  // Compile the shaders in parallel where the browser can, so the power-on
  // log keeps typing while the cabinet gets ready. Without the extension
  // they compile on the first frame instead.
  if (renderer.getContext().getExtension("KHR_parallel_shader_compile")) await renderer.compileAsync(scene, camera);

  return {
    /** Flickers the lamps on over `seconds`. */
    powerOn: (seconds) => {
      powerT = 0;
      powerLength = seconds;
    },
    tilt: (dir) => {
      stickTarget = 0.4 * Math.sign(dir);
      stickHold = 0.18;
    },
    leanStick,
    holdStick,
    /** The stick's stop, in radians. */
    stickStop: STICK_STOP,
    /** Lets go of the stick, which springs back to centre. */
    releaseStick: () => (stickHeld = null),
    press: (i) => (pressed[i] = 0.16),
    insertCoin: (cb) => {
      coinT = 0;
      coinDone = cb;
    },
    pick,
    hover,
    frame,
    update,
    setTheme,
    screenChanged: () => (screenTex.needsUpdate = true),
  };
}
