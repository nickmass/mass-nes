function fake_log(arg) {
	if (arg[0] != '[') {return;}
	var screen = eval(arg);
	postMessage(screen);
}

console.log = fake_log;

importScripts('main.js');
