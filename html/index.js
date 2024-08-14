import express from 'express'
import fs from 'fs'
import path from 'path'

let server = express();

function loadPage(file, context) {
    let base = fs.readFileSync('base.html', 'utf8')
    let page = fs.readFileSync(file + '.html', 'utf8')
    
    for (let k of Object.keys(context ?? {})) {
        page = page.replace(`{% ${k} %}`, context[k])
    }

    return base.replace('{% content %}', page) 
}

function loadTemplate(file, context) {
    let template = fs.readFileSync(file + '.html', 'utf8')
    for (let k of Object.keys(context ?? {})) {
        template = template.replaceAll(`{% ${k} %}`, context[k])
    }
    return template;
}

server.get('/', (req, res) => res.send(loadPage('index')))
server.get('/color', (req, res) => res.send(loadPage('color', {
    current_color: "#ffffff"
})))
server.get('/setup', (req, res) => res.send(loadPage('setup')))



server.get('/available-networks', (req, res) => res.send(['miamihouse', 'ViejosPrivateNetwork'].map(n => loadTemplate('templates/network-option', {ssid: n})).join('\n')))

server.listen(80)