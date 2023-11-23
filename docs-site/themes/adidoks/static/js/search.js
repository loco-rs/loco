var suggestions = document.getElementById('suggestions');
var userinput = document.getElementById('userinput');

document.addEventListener('keydown', inputFocus);

function inputFocus(e) {

  if (e.keyCode === 191
      && document.activeElement.tagName !== "INPUT"
      && document.activeElement.tagName !== "TEXTAREA") {
    e.preventDefault();
    userinput.focus();
  }

  if (e.keyCode === 27 ) {
    userinput.blur();
    suggestions.classList.add('d-none');
  }

}

document.addEventListener('click', function(event) {

  var isClickInsideElement = suggestions.contains(event.target);

  if (!isClickInsideElement) {
    suggestions.classList.add('d-none');
  }

});

/*
Source:
  - https://dev.to/shubhamprakash/trap-focus-using-javascript-6a3
*/

document.addEventListener('keydown',suggestionFocus);

function suggestionFocus(e){
  const focusableSuggestions= suggestions.querySelectorAll('a');
  if (suggestions.classList.contains('d-none')
      || focusableSuggestions.length === 0) {
    return;
  }
  const focusable= [...focusableSuggestions];
  const index = focusable.indexOf(document.activeElement);

  let nextIndex = 0;

  if (e.keyCode === 38) {
    e.preventDefault();
    nextIndex= index > 0 ? index-1 : 0;
    focusableSuggestions[nextIndex].focus();
  }
  else if (e.keyCode === 40) {
    e.preventDefault();
    nextIndex= index+1 < focusable.length ? index+1 : index;
    focusableSuggestions[nextIndex].focus();
  }

}

/*
Source:
  - https://github.com/nextapps-de/flexsearch#index-documents-field-search
  - https://raw.githack.com/nextapps-de/flexsearch/master/demo/autocomplete.html
  - http://elasticlunr.com/
  - https://github.com/getzola/zola/blob/master/docs/static/search.js
*/
(function(){
  var index = elasticlunr.Index.load(window.searchIndex);
  userinput.addEventListener('input', show_results, true);
  suggestions.addEventListener('click', accept_suggestion, true);
  
  function show_results(){
    var value = this.value.trim();
    var options = {
      bool: "OR",
      fields: {
        title: {boost: 2},
        body: {boost: 1},
      }
    };
    var results = index.search(value, options);

    var entry, childs = suggestions.childNodes;
    var i = 0, len = results.length;
    var items = value.split(/\s+/);
    suggestions.classList.remove('d-none');

    results.forEach(function(page) {
      if (page.doc.body !== '') {
        entry = document.createElement('div');

        entry.innerHTML = '<a href><span></span><span></span></a>';
  
        a = entry.querySelector('a'),
        t = entry.querySelector('span:first-child'),
        d = entry.querySelector('span:nth-child(2)');
        a.href = page.ref;
        t.textContent = page.doc.title;
        d.innerHTML = makeTeaser(page.doc.body, items);
  
        suggestions.appendChild(entry);
      }
    });

    while(childs.length > len){
        suggestions.removeChild(childs[i])
    }

  }

  function accept_suggestion(){

      while(suggestions.lastChild){

          suggestions.removeChild(suggestions.lastChild);
      }

      return false;
  }

  // Taken from mdbook
  // The strategy is as follows:
  // First, assign a value to each word in the document:
  //  Words that correspond to search terms (stemmer aware): 40
  //  Normal words: 2
  //  First word in a sentence: 8
  // Then use a sliding window with a constant number of words and count the
  // sum of the values of the words within the window. Then use the window that got the
  // maximum sum. If there are multiple maximas, then get the last one.
  // Enclose the terms in <b>.
  function makeTeaser(body, terms) {
    var TERM_WEIGHT = 40;
    var NORMAL_WORD_WEIGHT = 2;
    var FIRST_WORD_WEIGHT = 8;
    var TEASER_MAX_WORDS = 30;
  
    var stemmedTerms = terms.map(function (w) {
      return elasticlunr.stemmer(w.toLowerCase());
    });
    var termFound = false;
    var index = 0;
    var weighted = []; // contains elements of ["word", weight, index_in_document]
  
    // split in sentences, then words
    var sentences = body.toLowerCase().split(". ");
    for (var i in sentences) {
      var words = sentences[i].split(/[\s\n]/);
      var value = FIRST_WORD_WEIGHT;
      for (var j in words) {
        
        var word = words[j];
  
        if (word.length > 0) {
          for (var k in stemmedTerms) {
            if (elasticlunr.stemmer(word).startsWith(stemmedTerms[k])) {
              value = TERM_WEIGHT;
              termFound = true;
            }
          }
          weighted.push([word, value, index]);
          value = NORMAL_WORD_WEIGHT;
        }
  
        index += word.length;
        index += 1;  // ' ' or '.' if last word in sentence
      }
  
      index += 1;  // because we split at a two-char boundary '. '
    }
  
    if (weighted.length === 0) {
      if (body.length !== undefined && body.length > TEASER_MAX_WORDS * 10) {
        return body.substring(0, TEASER_MAX_WORDS * 10) + '...';
      } else {
        return body;
      }
    }
  
    var windowWeights = [];
    var windowSize = Math.min(weighted.length, TEASER_MAX_WORDS);
    // We add a window with all the weights first
    var curSum = 0;
    for (var i = 0; i < windowSize; i++) {
      curSum += weighted[i][1];
    }
    windowWeights.push(curSum);
  
    for (var i = 0; i < weighted.length - windowSize; i++) {
      curSum -= weighted[i][1];
      curSum += weighted[i + windowSize][1];
      windowWeights.push(curSum);
    }
  
    // If we didn't find the term, just pick the first window
    var maxSumIndex = 0;
    if (termFound) {
      var maxFound = 0;
      // backwards
      for (var i = windowWeights.length - 1; i >= 0; i--) {
        if (windowWeights[i] > maxFound) {
          maxFound = windowWeights[i];
          maxSumIndex = i;
        }
      }
    }
  
    var teaser = [];
    var startIndex = weighted[maxSumIndex][2];
    for (var i = maxSumIndex; i < maxSumIndex + windowSize; i++) {
      var word = weighted[i];
      if (startIndex < word[2]) {
        // missing text from index to start of `word`
        teaser.push(body.substring(startIndex, word[2]));
        startIndex = word[2];
      }
  
      // add <em/> around search terms
      if (word[1] === TERM_WEIGHT) {
        teaser.push("<b>");
      }

      startIndex = word[2] + word[0].length;
      // Check the string is ascii characters or not
      var re = /^[\x00-\xff]+$/
      if (word[1] !== TERM_WEIGHT && word[0].length >= 12 && !re.test(word[0])) {
        // If the string's length is too long, it maybe a Chinese/Japance/Korean article
        // if using substring method directly, it may occur error codes on emoji chars
        var strBefor = body.substring(word[2], startIndex);
        var strAfter = substringByByte(strBefor, 12);
        teaser.push(strAfter);
      } else {
        teaser.push(body.substring(word[2], startIndex));
      }
  
      if (word[1] === TERM_WEIGHT) {
        teaser.push("</b>");
      }
    }
    teaser.push("…");
    return teaser.join("");
  }
}());


// Get substring by bytes
// If using JavaScript inline substring method, it will return error codes 
// Source: https://www.52pojie.cn/thread-1059814-1-1.html
function substringByByte(str, maxLength) {
  var result = "";
  var flag = false;
  var len = 0;
  var length = 0;
  var length2 = 0;
  for (var i = 0; i < str.length; i++) {
    var code = str.codePointAt(i).toString(16);
    if (code.length > 4) {
      i++;
      if ((i + 1) < str.length) {
        flag = str.codePointAt(i + 1).toString(16) == "200d";
      }
    }
    if (flag) {
      len += getByteByHex(code);
      if (i == str.length - 1) {
        length += len;
        if (length <= maxLength) {
          result += str.substr(length2, i - length2 + 1);
        } else {
          break
        }
      }
    } else {
      if (len != 0) {
        length += len;
        length += getByteByHex(code);
        if (length <= maxLength) {
          result += str.substr(length2, i - length2 + 1);
          length2 = i + 1;
        } else {
          break
        }
        len = 0;
        continue;
      }
      length += getByteByHex(code);
      if (length <= maxLength) {
        if (code.length <= 4) {
          result += str[i]
        } else {
          result += str[i - 1] + str[i]
        }
        length2 = i + 1;
      } else {
        break
      }
    }
  }
  return result;
}

// Get the string bytes from binary
function getByteByBinary(binaryCode) {
  // Binary system, starts with `0b` in ES6
  // Octal number system, starts with `0` in ES5 and starts with `0o` in ES6
  // Hexadecimal, starts with `0x` in both ES5 and ES6
  var byteLengthDatas = [0, 1, 2, 3, 4];
  var len = byteLengthDatas[Math.ceil(binaryCode.length / 8)];
  return len;
}

// Get the string bytes from hexadecimal
function getByteByHex(hexCode) {
  return getByteByBinary(parseInt(hexCode, 16).toString(2));
}
