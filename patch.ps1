$p = "E:\AI\ques-src-20261119\agent-client\web\src\index.css"
$t = [IO.File]::ReadAllText($p)

$oldRocket = @'
/* 火箭：整条被一层火箭壳包住，从左边窜进来；落地壳碎掉，条自然露出来 */
.spot[data-fx='rocket'] .quick-bar {
  animation: rocket-fly 0.5s cubic-bezier(0.18, 0.9, 0.22, 1) both;
}

/* 壳：两头收、尾翼张开，正好把整条罩住 */
.spot[data-fx='rocket'] .shell {
  position: absolute;
  top: -13px;
  bottom: -13px;
  left: -16px;
  right: -40px;
  clip-path: polygon(0% 22%, 76% 0%, 100% 50%, 76% 100%, 0% 78%);
  background:
    linear-gradient(180deg, rgb(255 255 255 / 26%), rgb(255 255 255 / 4%) 46%, rgb(0 0 0 / 18%)),
    color-mix(in oklab, var(--brand) 30%, transparent);
  box-shadow:
    inset 0 0 0 1px color-mix(in oklab, var(--brand) 78%, #fff),
    0 0 40px -6px color-mix(in oklab, var(--brand) 55%, transparent);
  animation: shell-out 0.32s ease-out 0.34s both;
}

/* 碎屑：壳散成一把光点，各飞各的方向 */
.spot[data-fx='rocket'] .shard {
  position: absolute;
  border-radius: 2px;
  background: linear-gradient(180deg, #fff, color-mix(in oklab, var(--brand) 70%, transparent));
  box-shadow: 0 0 10px 2px color-mix(in oklab, var(--brand) 45%, transparent);
  animation: shard-fly 0.52s cubic-bezier(0.2, 0.7, 0.3, 1) calc(0.32s + var(--d)) both;
}

/* 尾焰：喷在壳的屁股后面 */
.spot[data-fx='rocket'] .flame {
  position: absolute;
  top: 50%;
  right: calc(100% + 34px);
  width: 130px;
  height: 14px;
  margin-top: -7px;
  border-radius: 999px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    color-mix(in oklab, var(--brand) 40%, #ff6a00) 42%,
    #ffb24d 76%,
    #fff 100%
  );
  filter: blur(3px);
  animation: flame-out 0.5s ease-in both;
}

/* 落地那一下的冲击波 */
.spot[data-fx='rocket'] .fx::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 18px;
  animation: rocket-burst 0.72s ease-out both;
}
'@

$newRocket = @'
/* 火箭：整条被一枚弹体包住从左边窜进来；落定那一刻弹体从中间裂成上下两半分开，条露出来 */
.spot[data-fx='rocket'] .quick-bar {
  animation: rocket-fly 0.5s cubic-bezier(0.18, 0.9, 0.22, 1) both;
}

/* 弹体：不透明，飞行时把条整个罩住 —— 条自己的边框和字一点都不透出来 */
.spot[data-fx='rocket'] .hull {
  position: absolute;
  left: -14px;
  right: -14px;
  top: -12px;
  bottom: -12px;
  z-index: 2;
  background: linear-gradient(
    180deg,
    color-mix(in oklab, var(--brand) 24%, #262e3b) 0%,
    #141a24 44%,
    color-mix(in oklab, var(--brand) 14%, #05070c) 100%
  );
  box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--brand) 52%, #93a0b8);
}

/* 上下两片：同尺寸各留一半，合起来是完整弹体；落定后各朝各的方向裂开 */
.spot[data-fx='rocket'] .hull.up {
  border-radius: 24px 9px 0 0;
  clip-path: inset(0 0 50% 0);
  animation: hull-up 0.44s cubic-bezier(0.3, 0.7, 0.2, 1) 0.48s both;
}

.spot[data-fx='rocket'] .hull.down {
  border-radius: 0 0 9px 24px;
  clip-path: inset(50% 0 0 0);
  animation: hull-down 0.44s cubic-bezier(0.3, 0.7, 0.2, 1) 0.48s both;
}

/* 弹头：捅在前面的尖，领路飞；落定后朝前甩出去 */
.spot[data-fx='rocket'] .nose {
  position: absolute;
  left: calc(100% + 12px);
  top: 50%;
  width: 54px;
  height: 40px;
  margin-top: -20px;
  z-index: 2;
  clip-path: polygon(0 0, 100% 50%, 0 100%);
  background: linear-gradient(90deg, #171d27, color-mix(in oklab, var(--brand) 30%, #0a0d13));
  animation: nose-out 0.42s cubic-bezier(0.3, 0.7, 0.2, 1) 0.48s both;
}

/* 尾翼：尾部上下两片，裂开时跟各自那一半走 */
.spot[data-fx='rocket'] .fin {
  position: absolute;
  left: -40px;
  width: 34px;
  height: 20px;
  z-index: 2;
  background: linear-gradient(90deg, color-mix(in oklab, var(--brand) 20%, #1b2230), #0a0d13);
}

.spot[data-fx='rocket'] .fin.up {
  top: -30px;
  clip-path: polygon(100% 0, 100% 100%, 0 100%);
  animation: hull-up 0.44s cubic-bezier(0.3, 0.7, 0.2, 1) 0.46s both;
}

.spot[data-fx='rocket'] .fin.down {
  bottom: -30px;
  clip-path: polygon(100% 0, 100% 100%, 0 0);
  animation: hull-down 0.44s cubic-bezier(0.3, 0.7, 0.2, 1) 0.46s both;
}

/* 尾焰与碎火由 WebGL 画在这一层：只发光、不挡点击，飞完自己停 */
.fx-canvas {
  position: absolute;
  left: -160px;
  top: -90px;
  width: calc(100% + 320px);
  height: calc(100% + 180px);
  z-index: 3;
  pointer-events: none;
}

/* 落定那一下的冲击波 */
.spot[data-fx='rocket'] .fx::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 18px;
  animation: rocket-burst 0.6s ease-out 0.46s both;
}
'@

if (-not $t.Contains($oldRocket)) { throw "rocket block not found" }
$t = $t.Replace($oldRocket, $newRocket)

$oldKeys = @'
/* 壳碎掉：亮一下、胀一点、然后没了 */
@keyframes shell-out {
  0% {
    opacity: 1;
    transform: scale(1);
    filter: brightness(1) blur(0);
  }
  35% {
    opacity: 1;
    transform: scale(1.02);
    filter: brightness(1.9) blur(0.4px);
  }
  100% {
    opacity: 0;
    transform: scale(1.06);
    filter: brightness(2.4) blur(3px);
  }
}

/* 碎屑：各朝各的方向飞出去，边飞边转边熄 */
@keyframes shard-fly {
  0% {
    opacity: 0;
    transform: none;
  }
  16% {
    opacity: 1;
  }
  100% {
    opacity: 0;
    transform: translate(var(--dx), var(--dy)) rotate(var(--rot)) scale(0.5);
  }
}

/* 尾焰：越飞越散，熄火即断 */
@keyframes flame-out {
  0% {
    opacity: 0;
    transform: scaleX(0.28);
  }
  16% {
    opacity: 1;
    transform: scaleX(1.12);
  }
  78% {
    opacity: 0.9;
    transform: scaleX(1);
  }
  100% {
    opacity: 0;
    transform: scaleX(0.2);
  }
}

/* 落地那一圈冲击波：火箭压住条头的那一刻散开 */
@keyframes rocket-burst {
  0%,
  42% {
    opacity: 0;
    box-shadow: 0 0 0 0 color-mix(in oklab, var(--brand) 62%, transparent);
  }
  50% {
    opacity: 1;
    box-shadow: 0 0 0 0 color-mix(in oklab, var(--brand) 58%, transparent);
  }
  100% {
    opacity: 0;
    box-shadow: 0 0 0 30px color-mix(in oklab, var(--brand) 0%, transparent);
  }
}
'@

$newKeys = @'
/* 弹体裂开：先亮一道缝，两半各往上下分开、错开一点角度，最后化掉 */
@keyframes hull-up {
  0% {
    opacity: 1;
    transform: none;
    filter: brightness(1);
  }
  16% {
    opacity: 1;
    transform: translateY(-3px);
    filter: brightness(1.85);
  }
  100% {
    opacity: 0;
    transform: translateY(-34px) rotate(-2.6deg);
    filter: brightness(1.15);
  }
}

@keyframes hull-down {
  0% {
    opacity: 1;
    transform: none;
    filter: brightness(1);
  }
  16% {
    opacity: 1;
    transform: translateY(3px);
    filter: brightness(1.85);
  }
  100% {
    opacity: 0;
    transform: translateY(32px) rotate(2.6deg);
    filter: brightness(1.15);
  }
}

/* 弹头：朝前甩出去 */
@keyframes nose-out {
  0% {
    opacity: 1;
    transform: none;
  }
  100% {
    opacity: 0;
    transform: translate(58px, -12px) rotate(9deg);
  }
}

/* 落定那一刻的冲击波 */
@keyframes rocket-burst {
  0% {
    opacity: 0.85;
    box-shadow: 0 0 0 0 color-mix(in oklab, var(--brand) 58%, transparent);
  }
  100% {
    opacity: 0;
    box-shadow: 0 0 0 34px color-mix(in oklab, var(--brand) 0%, transparent);
  }
}
'@

if (-not $t.Contains($oldKeys)) { throw "keyframes block not found" }
$t = $t.Replace($oldKeys, $newKeys)

[IO.File]::WriteAllText($p, $t)
"index.css patched"
