/* eslint-disable @typescript-eslint/no-var-requires */
const fs = require('fs');
const path = require('path');
const { parse } = require('csv-parse');

fs.readdir('./migrations', (err, files) => {
  if (err) throw err;
  const fileList = files
    .filter((file) => /.*\.csv$/.test(file))
    .sort((a, b) => a > b);
  let csvData = [];
  let sqlLines = [];
  fileList.forEach((fileName) => {
    if (fileName.split('!')[1].split('.')[1] !== 'csv') {
      return;
    }
    const index = parseInt(fileName.split('!')[0], 10) - 1;

    fs.createReadStream(path.join(__dirname, '../migrations/', fileName))
      .pipe(parse())
      .on('data', (csvrow) => {
        if (!csvData[index]) {
          csvData[index] = [];
        }
        csvData[index].push(csvrow);
      })
      .on('end', () => {
        let sqlLinesInner = [
          `INSERT INTO \`${fileName.split('!')[1].split('.')[0]}\` VALUES `,
        ];
        csvData[index].forEach((data, idx) => {
          if (idx === 0) return;
          const cols = data
            .map((col, idx) => {
              if (csvData[index][0][idx]?.startsWith('#')) {
                return null;
              }
              return `'${col.replace(`'`, `\\'`)}'`;
            })
            .filter((col) => col !== null);
          if (csvData[index].length === idx + 1) {
            sqlLinesInner.push(`(${cols})`);
          } else {
            sqlLinesInner.push(`(${cols}),`);
          }
        });
        sqlLines[index] = sqlLinesInner.join('');
        if (sqlLines.length === fileList.length) {
          fs.writeFile('./tmp.sql', sqlLines.join(';\n'), (err) => {
            if (err) throw err;
          });
        }
      });
  });
});
