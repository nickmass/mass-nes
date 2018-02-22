import webEmu from "../Cargo.toml";

((canvas, romlist, enableAudio, pauseMsg, loadMsg) => {
  let gfxCtx = canvas.getContext("2d");
  let audioCtx = new window.AudioContext();
  let keys = [];
  let pause = false;
  let frame = 0;
  let romList = "roms/romlist.txt";

  populatList(romList);

  function mapKey(key) {
    switch (key) {
    case "ArrowUp": return "Up";
    case "ArrowLeft": return "Left";
    case "ArrowRight": return "Right";
    case "ArrowDown": return "Down";
    case "z": return "A";
    case "x": return "B";
    case "Shift": return "Select";
    case "Enter": return "Start";
    }

    return false;
  }

  window.addEventListener('keydown', evt => {
    let key = mapKey(evt.key);
    if (key && keys.every(k => k != key)) {
      keys.push(key);
    }

    if (evt.key == " ") {
      pause = !pause;
      if (pause) {
        pauseMsg.innerHTML = "Paused";
      } else {
        pauseMsg.innerHTML = "";
      }
    }
  });

  window.addEventListener('keyup', evt => {
    let key = mapKey(evt.key);
    keys = keys.filter(k => k != key);
  });

  webEmu.addEventListener("screen", screen => {
    let img = gfxCtx.createImageData(256, 240);
    img.data.set(screen);
    gfxCtx.putImageData(img, 0, 0);
  });

  webEmu.addEventListener("audio", samples => {
    if (!enableAudio.checked) { return; }
    let buf = audioCtx.createBuffer(1, samples.length, samples.length * 60);
    let data = buf.getChannelData(0);
    data.set(samples);

    let source = audioCtx.createBufferSource();
    source.buffer = buf;
    source.connect(audioCtx.destination);

    source.start();
  });

  function emuLoop() {
    if(!pause) {
      webEmu.run_frame(keys);
      frame += 1;
      console.log("Frame", frame);
    }
    window.requestAnimationFrame(emuLoop);
  }

  function populatList(romList) {
    fetch(romList)
      .then(res => res.text())
      .then(text => {
        let files = text.split('\n')
            .map(line => {
              let parts = line.split('=');
              if (parts.length != 2) { return false; };
              return {name: parts[0].trim(), path: parts[1].trim()};
            })
            .filter(f => f)
            .sort((a, b) => {
              if (a.name > b.name) { return 1; }
              if (a.name < b.name) { return -1; }
              return 0;
            });

        let options = files
            .map(file => {
              let elem = document.createElement("option");
              elem.innerHTML = file.name;
              elem.setAttribute("value", file.path);
              return elem;
            });

        romlist.removeEventListener("change", selectChange);
        while (romlist.firstChild) {
          romlist.removeChild(romlist.firstChild);
        }

        let selectElem = document.createElement("option");
        selectElem.innerHTML = "Select a Rom";
        selectElem.setAttribute("value", "");
        romlist.appendChild(selectElem);
        options.forEach(opt => romlist.appendChild(opt));
        romlist.addEventListener("change", selectChange);
      });
  }

  function selectChange(evt) {
    let path = evt.target.value;
    if (path) {
      loadMsg.innerHTML = "Loading...";
      return loadRom(path);
    } else {
      loadMsg.innerHTML = "";
      return Promise.resolve();
    }
  }

  function loadRom(path) {
    return fetch(path)
      .then(res => res.blob())
      .then(blob => {
        return new Promise((resolve, reject) => {
          let reader = new FileReader();
          reader.addEventListener("loadend", () => resolve(reader.result));
          reader.addEventListener("error", () => reject(reader.error));
          reader.readAsArrayBuffer(blob);
        });
      })
      .then(rom => {
        let buf = new Uint8Array(rom);
        webEmu.load_rom(Array.from(buf));

        frame = 0;
        emuLoop();
      })
      .then(() => {
        loadMsg.innerHTML = "";
        canvas.removeAttribute("hidden");
      })
      .catch(error => {
        loadMsg.innerHTML = `Error: ${error}`;
      });
  }
})(document.getElementById("canvas"),
   document.getElementById("rom-list"),
   document.getElementById("enable-audio"),
   document.getElementById("pause-message"),
   document.getElementById("load-message"));
